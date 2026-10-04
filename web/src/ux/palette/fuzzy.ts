// A small fuzzy scorer for the command palette. Higher is better, 0 is no
// match. Exact and whole-word matches beat prefixes, prefixes beat word-start
// substrings and initials, which beat loose subsequences. Loose subsequences
// only count for queries of four characters or more (two letters match far
// too much by accident).

const SPLIT = /[\s\-_./·›&(),:]+/;
const SEP = /[\s\-_./·›&(),:]/;

export function normalise(s: string): string {
  return s
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '');
}

/** Score one query token against one text (both already normalised). */
export function scoreToken(q: string, t: string): number {
  if (!q || !t) return 0;
  if (t === q) return 1000;
  if (t.startsWith(q)) return 900 - Math.min(80, t.length - q.length);
  const words = t.split(SPLIT).filter(Boolean);
  if (words.includes(q)) return 860;
  if (words.some((w) => w.startsWith(q))) return 780 - Math.min(60, t.indexOf(q) >= 0 ? t.indexOf(q) : 60);
  const initials = words.map((w) => w[0]).join('');
  if (q.length >= 2 && initials.startsWith(q)) return 720;
  const at = t.indexOf(q);
  if (at >= 0 && q.length >= 3) return 600 - Math.min(100, at);
  if (q.length < 4) return 0;
  // Subsequence with bonuses for consecutive letters and word starts.
  let ti = 0;
  let score = 0;
  let run = 0;
  let first = -1;
  let last = -2;
  for (const c of q) {
    const found = t.indexOf(c, ti);
    if (found < 0) return 0;
    if (first < 0) first = found;
    const wordStart = found === 0 || SEP.test(t[found - 1]);
    run = found === last + 1 ? run + 1 : 0;
    score += 10 + (wordStart ? 14 : 0) + run * 6;
    last = found;
    ti = found + 1;
  }
  const spread = last - first + 1;
  // Letters scattered across a long sentence are a coincidence, not a match.
  if (spread > Math.max(q.length * 2.5, q.length + 6)) return 0;
  return Math.max(1, Math.min(480, score * 2 - Math.max(0, spread - q.length) * 2));
}

/** Score a whole query (space-separated tokens must all match) against fields with weights. */
export function scoreFields(query: string, fields: { text: string | undefined; w: number }[]): number {
  const tokens = normalise(query).split(/\s+/).filter(Boolean);
  if (tokens.length === 0) return 0;
  const texts = fields.filter((f) => f.text).map((f) => ({ t: normalise(f.text!), w: f.w }));
  // The whole query against each field first (phrases such as "size a battery").
  const phrase = normalise(query).trim();
  let best = 0;
  for (const f of texts) best = Math.max(best, scoreToken(phrase, f.t) * f.w);
  if (tokens.length === 1) return best;
  let total = 0;
  for (const tok of tokens) {
    let b = 0;
    for (const f of texts) b = Math.max(b, scoreToken(tok, f.t) * f.w);
    if (b === 0) return best;
    total += b;
  }
  return Math.max(best, (total / tokens.length) * 0.92);
}
