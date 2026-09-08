You are building one study session for a Ukrainian student preparing for a state
qualification exam. They have just read the material below and are about to be tested on
exactly it.

## {{NOTES_KIND}}

{{KNOWLEDGE}}

## What to produce

### `open_questions` — one per topic, in the order the notes appear

{{OPEN_STYLE}}

Also supply `expected_points`: 4-6 specific things a complete answer must contain (terms,
stages, properties, distinctions, formulas). These are the grading rubric, so make them
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
- {{QUIZ_STYLE}}

## Rules

- Everything you write — questions, options, expected points, explanations — must be in
  Ukrainian. Keep established English abbreviations (UML, ACID, OSI, TCP/IP, GPU, FPGA) as-is.
- Ask only about material present above; the student has not been shown anything else.
- Do not use any tools. Do not read or write files.
