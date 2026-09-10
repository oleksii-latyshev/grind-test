//! Thin wrapper around the Antigravity CLI (`agy`) — the project's only model access.
//!
//! Every call is non-interactive and schema-constrained; we read `structured_output`
//! from the JSON envelope and ignore the prose `response`.

use anyhow::{anyhow, bail, Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

mod tiers;

pub use tiers::{
    configure, list_models, models, ModelInfo, ModelSettings, ModelTier, DEFAULT_FAST,
    DEFAULT_SMART,
};

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
}
