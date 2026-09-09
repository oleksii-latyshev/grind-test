use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{Emitter, Manager, State};

use crate::generate::{
    self, GenerationEvent, Hint, KnowledgeBatchReport, QuizRequest, Selection, SessionMode,
    SessionPlan, SessionResult,
};
use crate::vault::knowledge::{self, KnowledgeNote};
use crate::vault::progress::{self, Attempt, AttemptResult, SubjectStats};
use crate::vault::quiz::{self, Quiz, QuizSummary};
use crate::vault::study::{self, StudyOverview, TopicStudy};
use crate::vault::syllabus::{self, Subject, Topic};
use crate::vault::paths;
use crate::vault::Vault;

/// Progress events for a running knowledge batch.
pub const GENERATION_EVENT: &str = "generation://progress";

pub struct AppState {
    /// Swappable at runtime: the user can point the app at a different folder without
    /// restarting it.
    vault: Mutex<Vault>,
    config_dir: PathBuf,
}

impl AppState {
    pub fn new(vault: Vault, config_dir: PathBuf) -> Self {
        Self {
            vault: Mutex::new(vault),
            config_dir,
        }
    }

    /// A lock poisoned by a panic elsewhere should not take the whole app down — the value
    /// behind it is just a path.
    pub fn vault(&self) -> Vault {
        self.vault
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn replace_vault(&self, root: PathBuf) {
        *self
            .vault
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Vault::new(root);
    }

    /// `$GRIND_VAULT` is itself an explicit choice, so a shell that sets it never sees the
    /// setup screen.
    fn configured(&self) -> bool {
        std::env::var_os("GRIND_VAULT").is_some()
            || crate::config::load(&self.config_dir).onboarded
    }

    /// Point the app at `root` and record that setup is done.
    fn adopt_vault(&self, root: PathBuf) -> CmdResult<()> {
        let mut config = crate::config::load(&self.config_dir);
        config.vault_root = Some(root.clone());
        config.onboarded = true;
        crate::config::save(&self.config_dir, &config).map_err(fail)?;
        self.replace_vault(root);
        Ok(())
    }
}

type CmdResult<T> = Result<T, String>;

fn fail(error: anyhow::Error) -> String {
    format!("{error:#}")
}

#[derive(Debug, Serialize)]
pub struct SubjectOverview {
    pub id: String,
    pub title: String,
    pub topic_count: usize,
    pub knowledge_count: usize,
    pub quiz_count: usize,
    pub accuracy_percent: u32,
}

#[derive(Debug, Serialize)]
pub struct SubjectDetail {
    pub subject: Subject,
    /// Topics that already have a generated knowledge note.
    pub knowledge_topic_ids: Vec<String>,
    pub quizzes: Vec<QuizSummary>,
    pub stats: SubjectStats,
    pub study: StudyOverview,
    /// Per-topic study state, keyed by topic id. Absent means never studied.
    pub topic_study: BTreeMap<String, TopicStudy>,
}

#[derive(Debug, Serialize)]
pub struct VaultInfo {
    pub root: String,
    /// The student has been through setup and said where the vault is. Until then the app
    /// must not read the vault path at all — see `describe`.
    pub configured: bool,
    /// The app can actually list the syllabus directory right now.
    pub readable: bool,
    pub subject_count: usize,
    /// Why it is not readable, phrased for the user.
    pub error: Option<String>,
    pub agy_binary: Option<String>,
}

fn agy_binary() -> Option<String> {
    crate::agy::resolve_binary()
        .ok()
        .map(|path| path.to_string_lossy().to_string())
}

/// Describe the vault, probing the filesystem only once the student has pointed at one.
///
/// The probe is what raises a macOS folder-access prompt, so before setup it must not
/// happen: the first system panel anyone sees should be the folder picker they asked for.
fn describe(state: &AppState) -> VaultInfo {
    let vault = state.vault();
    let configured = state.configured();
    if !configured {
        return VaultInfo {
            root: vault.root.to_string_lossy().to_string(),
            configured: false,
            readable: false,
            subject_count: 0,
            error: None,
            agy_binary: agy_binary(),
        };
    }
    let (readable, subject_count, error) = match paths::describe_vault(&vault.root) {
        Ok(count) => (true, count, None),
        Err(message) => (false, 0, Some(message)),
    };
    VaultInfo {
        root: vault.root.to_string_lossy().to_string(),
        configured: true,
        readable,
        subject_count,
        error,
        agy_binary: agy_binary(),
    }
}

fn knowledge_ids(vault: &Vault, topics: &[&Topic]) -> Vec<String> {
    topics
        .iter()
        .filter(|topic| knowledge::exists(vault, topic))
        .map(|topic| topic.id.clone())
        .collect()
}

#[tauri::command]
pub fn vault_info(state: State<'_, AppState>) -> VaultInfo {
    describe(&state)
}

/// Create a vault in a folder the student picks, with the directory skeleton and — when
/// there is no syllabus anywhere in it — one sample file showing the format.
#[tauri::command]
pub async fn create_vault(app: tauri::AppHandle) -> CmdResult<VaultInfo> {
    let state = app.state::<AppState>();
    let Some(parent) = pick_folder(&app, "Оберіть теку, де створити сховище").await? else {
        return Ok(describe(&state));
    };

    // Picking a folder that is already a vault means "use this one", not "nest another".
    let root = if parent.join("syllabus").is_dir() {
        parent
    } else {
        parent.join("grind-vault")
    };
    paths::create_vault(&root).map_err(|error| {
        format!("не вдалося створити сховище в «{}»: {error}", root.display())
    })?;
    state.adopt_vault(root)?;
    Ok(describe(&state))
}

/// Accept the folder the app already resolved — the app data dir in a packaged build, the
/// repository vault in development — without opening a picker.
#[tauri::command]
pub fn use_default_vault(state: State<'_, AppState>) -> CmdResult<VaultInfo> {
    let root = state.vault().root;
    paths::create_vault(&root)
        .map_err(|error| format!("не вдалося підготувати «{}»: {error}", root.display()))?;
    state.adopt_vault(root)?;
    Ok(describe(&state))
}

/// Let the user point the app at the vault through the system folder picker.
///
/// Beyond being the obvious way to move a vault, on macOS this is also the way *back* from a
/// denied folder-access prompt: choosing a folder in the system panel is an explicit grant,
/// so the user can see exactly what they are allowing.
#[tauri::command]
pub async fn choose_vault(app: tauri::AppHandle) -> CmdResult<VaultInfo> {
    let state = app.state::<AppState>();
    let Some(picked) = pick_folder(&app, "Оберіть теку сховища").await? else {
        // Cancelled: report what is configured now rather than treating it as an error.
        return Ok(describe(&state));
    };
    state.adopt_vault(paths::normalise_vault_choice(&picked))?;
    Ok(describe(&state))
}

/// The system folder panel. Picking a folder here is the macOS grant, which is why it is the
/// only place the app ever reaches outside its own data directory.
async fn pick_folder(app: &tauri::AppHandle, title: &str) -> CmdResult<Option<PathBuf>> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title(title)
        .pick_folder(move |picked| {
            let _ = tx.send(picked);
        });

    let picked = rx
        .await
        .map_err(|_| "вікно вибору теки закрилося".to_string())?;
    picked
        .map(|path| path.into_path().map_err(|error| error.to_string()))
        .transpose()
}

#[tauri::command]
pub fn list_subjects(state: State<'_, AppState>) -> CmdResult<Vec<SubjectOverview>> {
    let vault = state.vault();
    let vault = &vault;
    let subjects = syllabus::load_all(vault).map_err(fail)?;
    let mastery = progress::load_mastery(vault);

    Ok(subjects
        .into_iter()
        .map(|subject| {
            let topics: Vec<&Topic> = subject.topics().collect();
            let knowledge_count = knowledge_ids(vault, &topics).len();
            let stats = progress::subject_stats(&mastery, &subject.id, &topics, knowledge_count);
            SubjectOverview {
                id: subject.id.clone(),
                title: subject.title.clone(),
                topic_count: topics.len(),
                knowledge_count,
                quiz_count: quiz::list(vault, &subject.id).map(|q| q.len()).unwrap_or(0),
                accuracy_percent: stats.accuracy_percent,
            }
        })
        .collect())
}

#[tauri::command]
pub fn get_subject(state: State<'_, AppState>, subject_id: String) -> CmdResult<SubjectDetail> {
    let vault = state.vault();
    let vault = &vault;
    let subject = syllabus::load(vault, &subject_id).map_err(fail)?;
    let topics: Vec<&Topic> = subject.topics().collect();
    let ids = knowledge_ids(vault, &topics);
    let mastery = progress::load_mastery(vault);
    let stats = progress::subject_stats(&mastery, &subject_id, &topics, ids.len());
    let quizzes = quiz::list(vault, &subject_id).map_err(fail)?;

    // Only topics with a note can be studied, so the overview is scoped to those.
    let state = study::load(vault);
    let studiable: Vec<&Topic> = topics
        .iter()
        .copied()
        .filter(|topic| ids.contains(&topic.id))
        .collect();
    let overview = study::overview(&state, &subject_id, &studiable);
    let topic_study = topics
        .iter()
        .filter_map(|topic| {
            state
                .topics
                .get(&topic.id)
                .map(|entry| (topic.id.clone(), entry.clone()))
        })
        .collect();

    Ok(SubjectDetail {
        knowledge_topic_ids: ids,
        quizzes,
        stats,
        study: overview,
        topic_study,
        subject,
    })
}

#[tauri::command]
pub fn get_knowledge(
    state: State<'_, AppState>,
    subject_id: String,
    topic_id: String,
) -> CmdResult<Option<KnowledgeNote>> {
    let vault = state.vault();
    let vault = &vault;
    let subject = syllabus::load(vault, &subject_id).map_err(fail)?;
    let Some(topic) = subject.find_topic(&topic_id) else {
        return Err(format!("unknown topic '{topic_id}'"));
    };
    knowledge::read(vault, topic).map_err(fail)
}

#[tauri::command]
pub async fn generate_knowledge(
    app: tauri::AppHandle,
    subject_id: String,
    topic_ids: Option<Vec<String>>,
    force: Option<bool>,
    concurrency: Option<usize>,
) -> CmdResult<KnowledgeBatchReport> {
    let vault = app.state::<AppState>().vault();
    let emitter = app.clone();
    let sink: generate::EventSink = Arc::new(move |event: GenerationEvent| {
        let _ = emitter.emit(GENERATION_EVENT, event);
    });

    generate::knowledge_batch(
        &vault,
        &subject_id,
        topic_ids,
        force.unwrap_or(false),
        concurrency.unwrap_or(4),
        sink,
    )
    .await
    .map_err(fail)
}

#[tauri::command]
pub async fn generate_quiz(app: tauri::AppHandle, request: QuizRequest) -> CmdResult<Quiz> {
    let vault = app.state::<AppState>().vault();
    generate::quiz(&vault, request).await.map_err(fail)
}

#[tauri::command]
pub fn list_quizzes(state: State<'_, AppState>, subject_id: String) -> CmdResult<Vec<QuizSummary>> {
    quiz::list(&state.vault(), &subject_id).map_err(fail)
}

#[tauri::command]
pub fn get_quiz(
    state: State<'_, AppState>,
    subject_id: String,
    quiz_id: String,
) -> CmdResult<Quiz> {
    quiz::read(&state.vault(), &subject_id, &quiz_id).map_err(fail)
}

#[tauri::command]
pub fn submit_quiz(
    state: State<'_, AppState>,
    subject_id: String,
    quiz_id: String,
    selections: BTreeMap<String, Vec<usize>>,
    hints_used: Vec<String>,
) -> CmdResult<AttemptResult> {
    generate::grade(&state.vault(), &subject_id, &quiz_id, selections, hints_used).map_err(fail)
}

#[tauri::command]
pub async fn get_hint(
    app: tauri::AppHandle,
    subject_id: String,
    quiz_id: String,
    question_id: String,
) -> CmdResult<Hint> {
    let vault = app.state::<AppState>().vault();
    generate::hint(&vault, &subject_id, &quiz_id, &question_id)
        .await
        .map_err(fail)
}

#[tauri::command]
pub fn list_attempts(state: State<'_, AppState>, subject_id: Option<String>) -> Vec<Attempt> {
    progress::list_attempts(&state.vault(), subject_id.as_deref())
}

// ---------------------------------------------------------------------------
// study mode
// ---------------------------------------------------------------------------

/// Pick the topics and load their notes. Returns without calling a model, so the reader can
/// open on the first note while `prepare_session_questions` runs behind it.
///
/// The returned plan has no questions yet — an empty `quiz` is what marks a session as not
/// yet ready.
#[tauri::command]
pub fn plan_study_session(
    state: State<'_, AppState>,
    subject_id: String,
    size: Option<usize>,
    topic_ids: Option<Vec<String>>,
    mode: Option<SessionMode>,
    selection: Option<Selection>,
) -> CmdResult<SessionPlan> {
    generate::plan_session(
        &state.vault(),
        &subject_id,
        size.unwrap_or(3),
        topic_ids,
        mode.unwrap_or_default(),
        selection.unwrap_or_default(),
    )
    .map_err(fail)
}

/// The one fast-model call a session costs. Idempotent, so the client may fire it on every
/// mount — including when resuming a session that was abandoned before it finished.
#[tauri::command]
pub async fn prepare_session_questions(
    app: tauri::AppHandle,
    subject_id: String,
    session_id: String,
) -> CmdResult<SessionPlan> {
    let vault = app.state::<AppState>().vault();
    generate::prepare_questions(&vault, &subject_id, &session_id)
        .await
        .map_err(fail)
}

/// Sessions that were started but never graded, newest first.
#[tauri::command]
pub fn unfinished_sessions(
    state: State<'_, AppState>,
    subject_id: String,
) -> Vec<generate::SessionSummary> {
    generate::list_unfinished(&state.vault(), &subject_id)
}

/// Reopen a session exactly as it was generated, rather than paying for a new one.
#[tauri::command]
pub fn resume_study_session(
    state: State<'_, AppState>,
    subject_id: String,
    session_id: String,
) -> CmdResult<SessionPlan> {
    generate::load_session(&state.vault(), &subject_id, &session_id).map_err(fail)
}

#[tauri::command]
pub async fn finish_study_session(
    app: tauri::AppHandle,
    subject_id: String,
    session_id: String,
    open_answers: BTreeMap<String, String>,
    quiz_selections: BTreeMap<String, Vec<usize>>,
) -> CmdResult<SessionResult> {
    let vault = app.state::<AppState>().vault();
    generate::finish_session(
        &vault,
        &session_id,
        &subject_id,
        open_answers,
        quiz_selections,
    )
    .await
    .map_err(fail)
}

/// Called when the student finishes reading a note, so a topic they have opened but not yet
/// been tested on is visibly distinct from one they have never touched.
#[tauri::command]
pub fn mark_topic_read(state: State<'_, AppState>, topic_id: String) -> CmdResult<()> {
    study::mark_read(&state.vault(), &topic_id).map_err(fail)
}
