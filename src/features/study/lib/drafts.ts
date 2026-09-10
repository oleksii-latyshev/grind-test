/**
 * Written answers are kept locally while a session is in progress, so a crash, an
 * accidental exit or a quit does not throw away several paragraphs of typing.
 */
const PREFIX = 'grind:session-draft:';

export function readDraft(sessionId: string): Record<string, string> {
  try {
    const raw = window.localStorage.getItem(PREFIX + sessionId);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return {};
    return Object.fromEntries(
      Object.entries(parsed as Record<string, unknown>).filter(
        ([, value]) => typeof value === 'string',
      ),
    ) as Record<string, string>;
  } catch {
    return {};
  }
}

export function writeDraft(sessionId: string, answers: Record<string, string>) {
  try {
    window.localStorage.setItem(PREFIX + sessionId, JSON.stringify(answers));
  } catch {
    // Storage unavailable or full: typing must keep working regardless.
  }
}

export function clearDraft(sessionId: string) {
  try {
    window.localStorage.removeItem(PREFIX + sessionId);
  } catch {
    // Nothing to do; a stale draft is harmless.
  }
}
