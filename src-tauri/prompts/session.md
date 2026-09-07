You are building one study session for a Ukrainian student preparing for a state
qualification exam. They have just read the study notes below and are about to be tested on
exactly this material.

## Study notes

{{KNOWLEDGE}}

## What to produce

### `open_questions` — one per topic, in the order the notes appear

These imitate the oral/written exam, where the student gets three broad questions and has to
develop an answer of several paragraphs. So each question must be **broad enough to require
a structured answer** — a definition plus a mechanism, a classification, a comparison, or a
worked application — and never answerable in one word.

Also supply `expected_points`: 4-6 specific things a complete answer must contain (terms,
stages, properties, formulas, trade-offs). These are the grading rubric, so make them
concrete and drawn from the note, not generic.

### `quiz_questions` — exactly {{QUIZ_COUNT}} multiple-choice questions

Spread evenly across the topics, each carrying the `topic_id` of the note it came from.

- 4 options each. `type` is `single` for one correct option, `multi` when two or more are
  correct — make roughly one in five `multi`.
- `correct` holds the 0-based indices of the correct options.
- Wrong options must be plausible to someone who half-read the note: real terms from the
  same field, common confusions, subtly wrong definitions. Never use filler like "жодна з
  відповідей".
- Do not make the correct option systematically the longest, and vary which index is correct.
- `explanation`: 1-3 sentences on why the correct option is right.

## Rules

- Everything you write — questions, options, expected points, explanations — must be in
  Ukrainian. Keep established English abbreviations (UML, ACID, OSI, TCP/IP, GPU) as-is.
- Ask only about material present in the notes above.
- Do not use any tools. Do not read or write files.
