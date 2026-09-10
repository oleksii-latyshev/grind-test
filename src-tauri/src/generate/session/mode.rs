//! How a session trades depth for coverage, and how its topics are chosen.

use serde::{Deserialize, Serialize};

/// Beyond this the single generation call has to cover too much ground, and the reading
/// stretch stops being one sitting.
pub const MAX_SESSION_TOPICS: usize = 6;

/// A sprint reads digests rather than full notes, so more topics still fit in one sitting
/// and in one generation call.
pub const MAX_SPRINT_TOPICS: usize = 10;

/// How a session trades depth for coverage.
///
/// The two costly halves of a session are reading the note and writing the answer, and the
/// modes cut them in that order: `Balanced` keeps the full note but drops the essay, and
/// only `Sprint` gives up the note itself. Cutting the reading first is what makes a fast
/// session forgettable — you cannot recall a mechanism you were never shown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionMode {
    /// Full note, essay-length written answer, 4 quiz questions per topic.
    #[default]
    Full,
    /// Full note, recall as a short list, 4 quiz questions per topic.
    Balanced,
    /// Condensed note, recall as a short list, 3 quiz questions per topic.
    Sprint,
}

impl SessionMode {
    pub(super) fn max_topics(self) -> usize {
        match self {
            SessionMode::Full => MAX_SESSION_TOPICS,
            SessionMode::Balanced => 8,
            SessionMode::Sprint => MAX_SPRINT_TOPICS,
        }
    }

    /// Only a sprint trades away the body of the note.
    pub(super) fn reads_digest(self) -> bool {
        matches!(self, SessionMode::Sprint)
    }

    pub(super) fn questions_per_topic(self) -> usize {
        match self {
            SessionMode::Full | SessionMode::Balanced => 4,
            SessionMode::Sprint => 3,
        }
    }

    pub(super) fn notes_kind(self) -> &'static str {
        if self.reads_digest() {
            "Condensed notes (summary, key terms and common traps only)"
        } else {
            "Study notes"
        }
    }

    pub(super) fn open_style(self) -> &'static str {
        match self {
            SessionMode::Full => "These imitate the oral/written exam, where the student gets three broad questions and has to develop an answer of several paragraphs. So each question must be **broad enough to require a structured answer** — a definition plus a mechanism, a classification, a comparison, or a worked application — and never answerable in one word.",
            SessionMode::Balanced | SessionMode::Sprint => "Each asks the student to **recall the substance as a short list**, not to write prose. Phrase them so a telegraphic answer is clearly what you want, e.g. \"Перелічіть …\", \"Назвіть …\", \"Коротко зіставте …\". A complete answer should take 4-6 bullet points.",
        }
    }

    pub(super) fn quiz_style(self) -> &'static str {
        match self {
            SessionMode::Sprint => "Aim at the distinctions the student is most likely to confuse under time pressure, and keep `explanation` to one or two sentences — it is read at speed.",
            _ => "Vary what you ask for: definitions, classification, \"which statement is false\", ordering of stages, choosing the right method for a situation. `explanation`: 1-3 sentences on why the correct option is right.",
        }
    }

    /// Told to the grader, so a deliberately terse answer is not marked down for being
    /// terse — otherwise every fast-mode score would collapse and drag the ladder with it.
    pub(super) fn answer_style(self) -> &'static str {
        match self {
            SessionMode::Full => "The student was asked for a developed written answer of several paragraphs, as in the oral exam.",
            SessionMode::Balanced | SessionMode::Sprint => "The student was asked to recall the substance as a short list, not as prose. Judge only whether the required points are named and correct. Do not deduct for telegraphic style, missing introductions, sentence fragments or absent connective prose — that form was requested.",
        }
    }
}

/// How the topics for a session are chosen when they are not listed explicitly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Selection {
    /// Follow the review ladder (or, in a sprint, syllabus order over unseen topics).
    #[default]
    Scheduled,
    /// One topic from each part of the syllabus, the way an exam paper draws them.
    Spread,
}
