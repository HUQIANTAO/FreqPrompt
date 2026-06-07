/**
 * FreqPrompt — Shared Language Detection
 *
 * Single source of truth for CJK-based language detection.
 * Used by both main.ts and paraphrase.ts.
 *
 * Returns 'zh' if the text has >30% CJK characters, 'en' otherwise.
 */
export function detectLanguage(text: string): string {
  let cjk = 0;
  let total = 0;
  for (const c of text) {
    if (c.trim() === '') continue;
    total++;
    const code = c.codePointAt(0)!;
    if (
      (code >= 0x4e00 && code <= 0x9fff) ||
      (code >= 0x3400 && code <= 0x4dbf) ||
      (code >= 0x3000 && code <= 0x303f)
    ) {
      cjk++;
    }
  }
  return total > 0 && cjk / total > 0.3 ? 'zh' : 'en';
}
