/**
 * FreqPrompt — Shared UI Utilities
 */

/** HTML-escape a string for safe innerHTML usage. */
export function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

/** Escape a string for safe HTML attribute usage. */
export function escapeAttr(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
