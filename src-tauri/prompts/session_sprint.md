You are building a **rapid revision** session for a Ukrainian student who has days, not
weeks, before a state qualification exam. They have just skim-read the condensed notes below
and need to be tested on them fast.

This is not the deep-study format: questions must be answerable from memory in a minute or
two, not developed into an essay.

## Condensed notes

{{KNOWLEDGE}}

## What to produce

### `open_questions` — one per topic, in the order the notes appear

Each asks the student to **recall the substance as a short list**, not to write prose. Phrase
them so a telegraphic answer is clearly what you want, e.g. "Перелічіть …", "Назвіть …",
"Коротко зіставте …". A complete answer should take 4-6 bullet points.

`expected_points`: 4-6 specific things that answer must name — terms, stages, properties,
distinctions, a formula. These are the grading rubric, so make them concrete and drawn from
the note. Keep each one short: this is a checklist, not a model answer.

### `quiz_questions` — exactly {{QUIZ_COUNT}} multiple-choice questions

Spread evenly across the topics, each carrying the `topic_id` of the note it came from.

- 4 options each. `type` is `single` for one correct option, `multi` when two or more are
  correct — make roughly one in six `multi`, since the point here is speed.
- `correct` holds the 0-based indices of the correct options.
- Aim at the distinctions the student is most likely to confuse under time pressure: which
  term means which, which method suits which situation, which statement is false. Wrong
  options must be plausible — real terms from the same field, not filler.
- Do not make the correct option systematically the longest, and vary which index is correct.
- `explanation`: one or two sentences, no more. It is read at speed.

## Rules

- Everything you write must be in Ukrainian. Keep established English abbreviations (UML,
  ACID, OSI, TCP/IP, GPU, FPGA) as-is.
- Ask only about material present in the condensed notes above — the student has not seen
  anything else.
- Do not use any tools. Do not read or write files.
