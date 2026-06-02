/**
 * FreqPrompt v2 — UI module
 * Results rendering, history, theme, config, toast.
 */

import type { ApiConfig } from './paraphrase';
import type { OptimizeOutput } from './frequency';

/* ══════════════════════════════════════════
   TYPES
   ══════════════════════════════════════════ */

export interface HistoryEntry {
  id: string;
  timestamp: number;
  original: string;
  optimized: string;
  optimizedScore: number;
  originalScore: number;
}

/* ══════════════════════════════════════════
   CONFIG
   ══════════════════════════════════════════ */

export function loadConfig(): ApiConfig {
  return {
    baseUrl: localStorage.getItem('fp_baseurl') || '',
    apiKey: localStorage.getItem('fp_apikey') || '',
    modelId: localStorage.getItem('fp_modelid') || '',
  };
}

export function saveConfig(config: ApiConfig): void {
  localStorage.setItem('fp_baseurl', config.baseUrl);
  localStorage.setItem('fp_apikey', config.apiKey);
  localStorage.setItem('fp_modelid', config.modelId);
}

export function populateConfigForm(): void {
  const c = loadConfig();
  (document.getElementById('base-url') as HTMLInputElement).value = c.baseUrl;
  (document.getElementById('api-key') as HTMLInputElement).value = c.apiKey;
  (document.getElementById('model-id') as HTMLInputElement).value = c.modelId;
  updateConfigIndicator(c);
}

export function updateConfigIndicator(config: ApiConfig | null): void {
  const dot = document.getElementById('config-dot')!;
  const label = document.getElementById('config-label')!;
  if (config && config.baseUrl && config.apiKey && config.modelId) {
    dot.className = 'dot ok';
    label.textContent = config.modelId;
  } else {
    dot.className = 'dot warn';
    label.textContent = '未配置';
  }
}

/* ══════════════════════════════════════════
   SETTINGS MODAL
   ══════════════════════════════════════════ */

let _settingsOpen = false;
export function toggleSettings(show?: boolean): void {
  _settingsOpen = show ?? !_settingsOpen;
  const overlay = document.getElementById('settings-overlay')!;
  if (_settingsOpen) {
    populateConfigForm();
    overlay.classList.add('open');
  } else {
    overlay.classList.remove('open');
  }
  overlay.onclick = (e) => { if (e.target === overlay) toggleSettings(false); };
}

/* ══════════════════════════════════════════
   THEME
   ══════════════════════════════════════════ */

export function initTheme(): void {
  const saved = localStorage.getItem('fp_theme');
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
  const isDark = saved === 'dark' || (!saved && prefersDark);
  applyTheme(isDark);
}

export function toggleTheme(): void {
  const isDark = document.documentElement.classList.contains('dark');
  applyTheme(!isDark);
  localStorage.setItem('fp_theme', !isDark ? 'dark' : 'light');
}

function applyTheme(isDark: boolean): void {
  const root = document.documentElement;
  if (isDark) {
    root.classList.add('dark');
    (document.getElementById('icon-sun') as HTMLElement).style.display = 'none';
    (document.getElementById('icon-moon') as HTMLElement).style.display = '';
  } else {
    root.classList.remove('dark');
    (document.getElementById('icon-sun') as HTMLElement).style.display = '';
    (document.getElementById('icon-moon') as HTMLElement).style.display = 'none';
  }
}

/* ══════════════════════════════════════════
   HISTORY (localStorage)
   ══════════════════════════════════════════ */

const MAX_HISTORY = 50;

export function loadHistory(): HistoryEntry[] {
  try {
    const raw = localStorage.getItem('fp_history');
    return raw ? (JSON.parse(raw) as HistoryEntry[]) : [];
  } catch { return []; }
}

function saveHistory(entries: HistoryEntry[]): void {
  localStorage.setItem('fp_history', JSON.stringify(entries.slice(0, MAX_HISTORY)));
}

export function addHistoryEntry(original: string, result: OptimizeOutput): void {
  const entries = loadHistory();
  entries.unshift({
    id: Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
    timestamp: Date.now(),
    original,
    optimized: result.optimized.text,
    optimizedScore: result.optimized.zipf_score,
    originalScore: result.original.zipf_score,
  });
  saveHistory(entries);
  updateHistoryBadge();
}

export function renderHistory(): void {
  const container = document.getElementById('history-list')!;
  const entries = loadHistory();

  if (entries.length === 0) {
    container.innerHTML = '<div class="history-empty">还没有优化记录</div>';
    return;
  }

  container.innerHTML = entries.map(e => {
    const time = new Date(e.timestamp);
    const timeStr = time.toLocaleString('zh-CN', {
      month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
    });
    return `
      <div class="history-item" data-id="${e.id}">
        <div class="history-item-time">${timeStr}</div>
        <div class="history-item-prompt">${escapeHtml(e.original.slice(0, 80))}</div>
        <div class="history-item-score">
          ${e.originalScore.toFixed(2)} → ${e.optimizedScore.toFixed(2)}
          <span style="color:${e.optimizedScore > e.originalScore ? 'var(--success)' : 'var(--text-muted)'};margin-left:4px;">
            ${e.optimizedScore > e.originalScore ? '↑' : '—'}
          </span>
        </div>
      </div>`;
  }).join('');
}

export function updateHistoryBadge(): void {
  const badge = document.querySelector('#btn-history .badge-dot');
  const entries = loadHistory();
  if (badge) {
    (badge as HTMLElement).style.display = entries.length > 0 ? '' : 'none';
  }
}

/* ══════════════════════════════════════════
   HISTORY DRAWER
   ══════════════════════════════════════════ */

let _drawerOpen = false;
export function toggleHistory(show?: boolean): void {
  _drawerOpen = show ?? !_drawerOpen;
  const overlay = document.getElementById('history-overlay')!;
  const drawer = document.getElementById('history-drawer')!;
  if (_drawerOpen) {
    renderHistory();
    overlay.classList.add('open');
    drawer.classList.add('open');
  } else {
    overlay.classList.remove('open');
    drawer.classList.remove('open');
  }
  overlay.onclick = () => toggleHistory(false);
}

/* ══════════════════════════════════════════
   TOAST
   ══════════════════════════════════════════ */

let _toastTimer: ReturnType<typeof setTimeout> | null = null;
export function showToast(message: string): void {
  const el = document.getElementById('toast')!;
  el.textContent = message;
  el.classList.add('show');
  if (_toastTimer) clearTimeout(_toastTimer);
  _toastTimer = setTimeout(() => el.classList.remove('show'), 2000);
}

/* ══════════════════════════════════════════
   COPY (proper implementation)
   ══════════════════════════════════════════ */

export async function copyText(text: string, buttonEl?: HTMLElement): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    if (buttonEl) {
      const orig = buttonEl.textContent;
      buttonEl.textContent = '已复制';
      buttonEl.style.color = 'var(--success)';
      setTimeout(() => {
        buttonEl.textContent = orig;
        buttonEl.style.color = '';
      }, 1800);
    }
    showToast('已复制到剪贴板');
  } catch {
    // Fallback
    const ta = document.createElement('textarea');
    ta.value = text;
    ta.style.position = 'fixed'; ta.style.opacity = '0';
    document.body.appendChild(ta);
    ta.select();
    document.execCommand('copy');
    document.body.removeChild(ta);
    showToast('已复制到剪贴板');
  }
}

/* ══════════════════════════════════════════
   ERROR / LOADING
   ══════════════════════════════════════════ */

export function showError(message: string): void {
  const el = document.getElementById('error-msg')!;
  el.textContent = message;
  el.style.display = 'block';
}

export function hideError(): void {
  document.getElementById('error-msg')!.style.display = 'none';
}

export function setLoading(loading: boolean): void {
  const btn = document.getElementById('optimize-btn') as HTMLButtonElement;
  const spinner = document.getElementById('optimize-spinner')!;
  btn.disabled = loading;
  spinner.style.display = loading ? 'inline-flex' : 'none';
}

/* ══════════════════════════════════════════
   CHAR COUNT
   ══════════════════════════════════════════ */

export function updateCharCount(): void {
  const ta = document.getElementById('prompt-input') as HTMLTextAreaElement;
  const count = ta.value.length;
  document.getElementById('char-count')!.textContent = `${count} 字符`;
}

/* ══════════════════════════════════════════
   SCORE HELPERS
   ══════════════════════════════════════════ */

function scoreColor(score: number): string {
  if (score >= 5.0) return 'score-high';
  if (score >= 4.0) return 'score-mid';
  return 'score-low';
}

function deltaClass(d: number): string {
  if (d > 0.05) return 'delta-pos';
  if (d < -0.05) return 'delta-neg';
  return 'delta-neu';
}

function fmtDelta(d: number): string {
  if (d > 0.01) return `+${d.toFixed(2)}`;
  if (d < -0.01) return d.toFixed(2);
  return '±0.00';
}

/* ── SVG Score Ring ── */
function renderScoreRing(score: number, max: number = 8): string {
  const pct = Math.max(0.02, Math.min(score / max, 1));
  const r = 28;
  const circ = 2 * Math.PI * r;
  const dash = pct * circ;
  let stroke = 'var(--error)';
  if (pct >= 0.625) stroke = 'var(--success)';
  else if (pct >= 0.5) stroke = 'var(--warning)';

  return `
    <svg width="72" height="72" viewBox="0 0 72 72">
      <circle cx="36" cy="36" r="${r}" fill="none" stroke="var(--hairline)" stroke-width="5"/>
      <circle cx="36" cy="36" r="${r}" fill="none" stroke="${stroke}" stroke-width="5"
        stroke-dasharray="${dash.toFixed(1)} ${(circ - dash).toFixed(1)}"
        stroke-linecap="round" transform="rotate(-90 36 36)"
        style="transition: stroke-dasharray 0.6s cubic-bezier(0.4,0,0.2,1);"/>
      <text x="36" y="38" text-anchor="middle"
        font-family="JetBrains Mono, monospace" font-size="16" font-weight="600"
        fill="var(--text)">${score.toFixed(1)}</text>
    </svg>`;
}

/* ══════════════════════════════════════════
   RESULTS RENDERING
   ══════════════════════════════════════════ */

export function renderResults(result: OptimizeOutput): void {
  const empty = document.getElementById('empty-state')!;
  const content = document.getElementById('results-content')!;
  empty.style.display = 'none';
  content.style.display = 'block';

  const delta = result.optimized.zipf_score - result.original.zipf_score;
  const optPct = Math.round((result.optimized.zipf_score / 8) * 100);
  const origPct = Math.round((result.original.zipf_score / 8) * 100);

  let html = '';

  // ── Score hero ──
  html += `
    <div class="score-hero">
      <div class="score-ring-wrap">${renderScoreRing(result.optimized.zipf_score)}</div>
      <div class="score-info">
        <div class="score-label">优化后得分 · Zipf 标度 (0–8)</div>
        <div class="score-value">${result.optimized.zipf_score.toFixed(2)}</div>
        <div class="score-delta ${deltaClass(delta)}">
          ${fmtDelta(delta)} vs 原始
          <span style="font-size:11px;font-weight:400;color:var(--text-muted);">
            (${result.original.zipf_score.toFixed(2)})
          </span>
        </div>
        <div class="score-bar-wrap">
          <div class="score-bar-fill orig" style="width:${origPct}%;" title="原始: ${result.original.zipf_score.toFixed(2)}"></div>
          <div class="score-bar-fill opt" style="width:${optPct - origPct}%;" title="提升: +${delta.toFixed(2)}"></div>
        </div>
      </div>
    </div>`;

  // ── Side-by-side score comparison ──
  html += `
    <div class="score-compare">
      <div class="score-compare-item">
        <div class="score-compare-label">原始</div>
        <div class="score-compare-value" style="color:var(--text-muted);">${result.original.zipf_score.toFixed(2)}</div>
        <div class="score-compare-bar" style="width:${origPct}%;background:var(--text-soft);"></div>
      </div>
      <div class="score-compare-item">
        <div class="score-compare-label">优化后</div>
        <div class="score-compare-value" style="color:var(--brand);">${result.optimized.zipf_score.toFixed(2)}</div>
        <div class="score-compare-bar" style="width:${optPct}%;background:var(--brand);"></div>
      </div>
    </div>`;

  // ── Optimized prompt card ──
  html += `
    <div class="prompt-card" id="opt-card">
      <div class="prompt-card-header">
        <span class="prompt-card-label">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px;margin-right:4px;"><path d="M12 3l1.5 5.5L19 10l-5.5 1.5L12 17l-1.5-5.5L5 10l5.5-1.5z"/></svg>
          优化后 Prompt
        </span>
        <span class="prompt-card-badge badge-best">最佳</span>
      </div>
      <div class="prompt-card-body">${escapeHtml(result.optimized.text)}</div>
      <div class="prompt-card-actions">
        <button class="btn btn-sm js-copy" data-text="${escapeAttr(result.optimized.text)}">复制</button>
        <button class="btn btn-ghost btn-sm js-use-as-input" data-text="${escapeAttr(result.optimized.text)}">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 14 4 9 9 4"/><path d="M20 20v-7a4 4 0 0 0-4-4H4"/></svg>
          作为新输入
        </button>
      </div>
    </div>`;

  // ── Original prompt card ──
  html += `
    <div class="prompt-card">
      <div class="prompt-card-header">
        <span class="prompt-card-label">原始 Prompt</span>
        <span class="prompt-card-badge badge-orig">原始</span>
      </div>
      <div class="prompt-card-body is-original">${escapeHtml(result.original.text)}</div>
    </div>`;

  // ── All candidates ──
  html += `<div class="candidates-section">`;
  html += `<div class="candidates-header">
    <span class="candidates-label">所有候选版本</span>
    <span class="candidates-count">${result.candidates.length} 个</span>
  </div>`;

  for (let i = 0; i < result.candidates.length; i++) {
    const c = result.candidates[i];
    const isOpt = c.text === result.optimized.text;
    const isOrig = c.text === result.original.text;
    let cls = '';
    if (isOpt) cls = 'is-best';
    else if (isOrig) cls = 'is-original';

    let tag = '';
    if (isOpt) tag = '<span class="tag-inline best">最佳</span>';
    else if (isOrig) tag = '<span class="tag-inline orig">原始</span>';

    html += `
      <div class="candidate-row ${cls}">
        <span class="candidate-rank">#${i + 1}</span>
        <span class="candidate-text">${escapeHtml(c.text)}${tag}</span>
        <span class="candidate-score ${scoreColor(c.zipf_score)}">${c.zipf_score.toFixed(2)}</span>
      </div>`;
  }
  html += `</div>`;

  content.innerHTML = html;

  // Bind event listeners to dynamically created buttons
  bindResultActions(result);
}

/** Attach proper event listeners to result buttons */
function bindResultActions(result: OptimizeOutput): void {
  // Copy buttons
  document.querySelectorAll('.js-copy').forEach(btn => {
    btn.addEventListener('click', () => {
      const text = (btn as HTMLElement).dataset.text || '';
      copyText(text, btn as HTMLElement);
    });
  });

  // "Use as input" buttons
  document.querySelectorAll('.js-use-as-input').forEach(btn => {
    btn.addEventListener('click', () => {
      const text = (btn as HTMLElement).dataset.text || '';
      const ta = document.getElementById('prompt-input') as HTMLTextAreaElement;
      ta.value = text;
      ta.focus();
      updateCharCount();
      showToast('已填入输入框');
      // Scroll input into view on mobile
      ta.scrollIntoView({ behavior: 'smooth', block: 'center' });
    });
  });
}

/* ══════════════════════════════════════════
   UTILS
   ══════════════════════════════════════════ */

export function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function escapeAttr(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
