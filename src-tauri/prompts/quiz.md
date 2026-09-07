You are writing a practice test for a Ukrainian student preparing for a state qualification
exam. Generate exactly {{QUESTION_COUNT}} multiple-choice questions.

## Source material

Every question MUST be answerable from the study notes below, and MUST carry the `topic_id`
of the note it came from. Do not ask about anything outside these notes.

{{KNOWLEDGE}}

## Coverage and difficulty

Requested difficulty: {{DIFFICULTY}}
Spread the questions across the supplied topics; these ones deserve extra attention because
the student has been getting them wrong:
{{WEAK_TOPICS}}

Do not re-use, and do not merely reword, any of these recently asked questions:
{{RECENT_QUESTIONS}}

## Question rules

- 4 options each. `type` is `single` for one correct option, `multi` when two or more are
  correct — make roughly one in five questions `multi`.
- `correct` holds the 0-based indices of the correct options.
- Wrong options must be plausible to someone who half-knows the topic: real terms from the
  same field, common confusions, subtly wrong definitions. Never use filler like "жодна з
  відповідей" or obviously absurd options.
- Vary what you ask for: definitions, classification, "which statement is false", ordering of
  stages, choosing the right method for a situation, reading a short formula.
- Do not make the correct option systematically the longest or most detailed one, and vary
  which index is correct.
- `explanation`: 1-3 sentences saying why the correct option is correct and, when useful, why
  the most tempting wrong one is wrong.

## Rules

- Write ALL questions, options and explanations in Ukrainian. Keep established English
  abbreviations as-is.
- Do not use any tools. Do not read or write files.
