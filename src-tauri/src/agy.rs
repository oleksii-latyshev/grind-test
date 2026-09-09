//! Thin wrapper around the Antigravity CLI (`agy`) — the project's only model access.
//!
//! Every call is non-interactive and schema-constrained; we read `structured_output`
//! from the JSON envelope and ignore the prose `response`.

use anyhow::{anyhow, bail, Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::RwLock;
use std::time::Duration;
use tokio::process::Command;

/// Which model a job is allowed to use. Ids live here and nowhere else.
///
/// The *tier* a job runs at is fixed by the job — knowledge and hints are always smart,
/// quizzes and sessions always fast — and only which model each tier names is configurable.
/// That keeps the economy rule enforceable while letting the student pick the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    /// Deep reasoning: knowledge notes and hints. Expensive, results are cached on disk.
    Smart,
    /// Cheap and quick: quiz and session generation, which happens constantly.
    Fast,
}

pub const DEFAULT_SMART: &str = "gemini-3.1-pro-high";
pub const DEFAULT_FAST: &str = "gemini-3.8-flash-medium";

/// The model id each tier resolves to. Anything `agy models` lists is valid, which is how
/// Claude, Gemini and the rest are all reachable without the app knowing about providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSettings {
    pub smart: String,
    pub fast: String,
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            smart: DEFAULT_SMART.to_string(),
            fast: DEFAULT_FAST.to_string(),
        }
    }
}

/// Process-wide, so the dozens of call sites that ask for a tier stay unaware of settings.
/// Written once at startup and again whenever the settings screen saves.
static MODELS: RwLock<Option<ModelSettings>> = RwLock::new(None);

pub fn configure(settings: ModelSettings) {
    *MODELS
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(settings);
}

pub fn models() -> ModelSettings {
    MODELS
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
        .unwrap_or_default()
}

impl ModelTier {
    pub fn model_id(self) -> String {
        let settings = models();
        match self {
            ModelTier::Smart => settings.smart,
            ModelTier::Fast => settings.fast,
        }
    }
}

/// One entry from `agy models`.
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub label: String,
}

/// Ask the CLI what it can run, so the settings screen offers real ids rather than a list
/// this app would have to keep in step with Antigravity by hand.
pub async fn list_models() -> Result<Vec<ModelInfo>> {
    let binary = resolve_binary()?;
    let output = tokio::time::timeout(
        Duration::from_secs(60),
        Command::new(&binary)
            .arg("models")
            .stdin(Stdio::null())
            .output(),
    )
    .await
    .map_err(|_| anyhow!("`agy models` timed out"))?
    .with_context(|| format!("running {} models", binary.display()))?;

    if !output.status.success() {
        bail!(
            "`agy models` exited with {}: {}",
            output.status,
            truncate(String::from_utf8_lossy(&output.stderr).trim(), 300)
        );
    }
    let models = parse_models(&String::from_utf8_lossy(&output.stdout));
    if models.is_empty() {
        bail!("`agy models` listed nothing");
    }
    Ok(models)
}

/// `id<TAB>Human label`, one per line, after a line or two of progress chatter.
fn parse_models(stdout: &str) -> Vec<ModelInfo> {
    stdout
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter_map(|(id, label)| {
            let id = id.trim();
            (!id.is_empty()).then(|| ModelInfo {
                id: id.to_string(),
                label: label.trim().to_string(),
            })
        })
        .collect()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    #[serde(default)]
    status: String,
    #[serde(default)]
    structured_output: Option<serde_json::Value>,
    #[serde(default)]
    usage: Usage,
    #[serde(default)]
    duration_seconds: f64,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Response<T> {
    pub data: T,
    pub usage: Usage,
    pub duration_seconds: f64,
    pub model: String,
}

/// Locate the `agy` binary. A bundled macOS `.app` does not inherit the login shell `PATH`,
/// so falling back to the usual install locations is not optional.
pub fn resolve_binary() -> Result<PathBuf> {
    if let Some(explicit) = std::env::var_os("GRIND_AGY_BIN") {
        let path = PathBuf::from(explicit);
        if path.is_file() {
            return Ok(path);
        }
        bail!(
            "GRIND_AGY_BIN points at {}, which is not a file",
            path.display()
        );
    }

    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join("agy");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    let home = std::env::var_os("HOME").map(PathBuf::from);
    let fallbacks = [
        home.as_ref().map(|h| h.join(".local/bin/agy")),
        Some(PathBuf::from("/opt/homebrew/bin/agy")),
        Some(PathBuf::from("/usr/local/bin/agy")),
    ];
    for candidate in fallbacks.into_iter().flatten() {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    bail!("`agy` not found. Install the Antigravity CLI or set GRIND_AGY_BIN to its path.")
}

/// Run one prompt and deserialize the model's structured output into `T`.
///
/// Retries once — these calls are long and a transient CLI failure should not lose a whole
/// batch — then gives up.
pub async fn run<T: DeserializeOwned>(
    prompt: &str,
    tier: ModelTier,
    schema: &str,
    timeout: Duration,
) -> Result<Response<T>> {
    match run_once(prompt, tier, schema, timeout).await {
        Ok(response) => Ok(response),
        Err(first) => run_once(prompt, tier, schema, timeout)
            .await
            .map_err(|second| anyhow!("agy failed twice: {first}; then: {second}")),
    }
}

async fn run_once<T: DeserializeOwned>(
    prompt: &str,
    tier: ModelTier,
    schema: &str,
    timeout: Duration,
) -> Result<Response<T>> {
    let binary = resolve_binary()?;

    // `agy` is an agent with file tools and it reads CLAUDE.md/AGENTS.md from its cwd.
    // Run it in a throwaway directory so it can neither see nor touch the repo.
    let scratch = std::env::temp_dir().join(format!("grind-agy-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&scratch)
        .with_context(|| format!("creating scratch dir {}", scratch.display()))?;
    let _guard = ScratchDir(scratch.clone());

    let schema_path = scratch.join("schema.json");
    std::fs::write(&schema_path, schema).context("writing json schema")?;

    let mut command = Command::new(&binary);
    command
        .current_dir(&scratch)
        .arg("--print")
        .arg(prompt)
        .args(["--model", &tier.model_id()])
        .args(["--output-format", "json"])
        .arg("--json-schema")
        .arg(&schema_path)
        .args(["--print-timeout", &format!("{}s", timeout.as_secs())])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = command
        .spawn()
        .with_context(|| format!("spawning {}", binary.display()))?;

    // The CLI has its own timeout; ours is the outer backstop in case it wedges.
    let output =
        match tokio::time::timeout(timeout + Duration::from_secs(30), child.wait_with_output())
            .await
        {
            Ok(result) => result.context("waiting for agy")?,
            Err(_) => bail!("agy timed out after {:?}", timeout),
        };

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "agy exited with {}: {}",
            output.status,
            truncate(stderr.trim(), 500)
        );
    }

    let envelope: Envelope = parse_envelope(&stdout)?;
    if envelope.status != "SUCCESS" {
        bail!(
            "agy returned status {}: {}",
            envelope.status,
            envelope.error.unwrap_or_default()
        );
    }

    let value = envelope
        .structured_output
        .ok_or_else(|| anyhow!("agy returned no structured_output"))?;
    let data = serde_json::from_value(value.clone()).with_context(|| {
        format!(
            "structured_output did not match the requested schema: {}",
            truncate(&value.to_string(), 500)
        )
    })?;

    Ok(Response {
        data,
        usage: envelope.usage,
        duration_seconds: envelope.duration_seconds,
        model: tier.model_id(),
    })
}

/// The envelope is the last JSON object on stdout; anything before it is CLI chatter.
fn parse_envelope(stdout: &str) -> Result<Envelope> {
    let line = stdout
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('{'))
        .ok_or_else(|| anyhow!("agy produced no JSON: {}", truncate(stdout.trim(), 500)))?;
    serde_json::from_str(line.trim())
        .with_context(|| format!("parsing agy output: {}", truncate(line, 500)))
}

fn truncate(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        None => text.to_string(),
        Some((cut, _)) => format!("{}…", &text[..cut]),
    }
}

struct ScratchDir(PathBuf);

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_last_json_line() {
        let out = "warming up\n{\"status\":\"SUCCESS\",\"structured_output\":{\"a\":1}}\n";
        let envelope = parse_envelope(out).unwrap();
        assert_eq!(envelope.status, "SUCCESS");
        assert!(envelope.structured_output.is_some());
    }

    #[test]
    fn reports_missing_json() {
        assert!(parse_envelope("command not found\n").is_err());
    }

    #[test]
    fn reads_the_model_listing() {
        let listing = "Fetching available models...\ngemini-3.1-pro-high\tGemini 3.1 Pro (High)\nclaude-opus-4-6-thinking\tClaude Opus 4.6 (Thinking)\n";
        let models = parse_models(listing);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "gemini-3.1-pro-high");
        assert_eq!(models[1].label, "Claude Opus 4.6 (Thinking)");
    }

    #[test]
    fn tiers_default_to_the_shipped_ids() {
        let settings = ModelSettings::default();
        assert_eq!(settings.smart, DEFAULT_SMART);
        assert_eq!(settings.fast, DEFAULT_FAST);
    }
}
