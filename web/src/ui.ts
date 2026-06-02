/**
 * FreqPrompt v2 — UI module
 * Results rendering, history, theme, config, toast, round progress.
 */
import type { ApiConfig } from './paraphrase';
import type { OptimizeOutput, FrequencyResult } from './frequency';

/* ════════════════════════════════════
   TYPES
   ════════════════════════════════════ */

export interface HistoryEntry {
  id: string;
  timestamp: number;
  original: string;
  optimized: string;
  optimizedScore: number;
  originalScore: number;
  language: string;
}

export interface RoundMeta {
  r1Delta: number;
  r2Applied: boolean;
  beamWidth?: number;
}

export interface LiveScore {
  avg: number;
  tokens: { text: string; zipf_score: number }[];
}

/* ════════════════════════════════════
   CONFIG
   ════════════════════════════════════ */

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

/* ════════════════════════════════════
   ROUND PROGRESS
   ════════════════════════════════════ */

export function showRoundProgress(message: string): void {
  const el = document.getElementById('round-progress')!;
  if (message) {
    el.textContent = message;
    el.style.display = 'block';
  } else {
    el.style.display = 'none';
  }
}

/* ════════════════════════════════════
   SETTINGS MODAL
   ════════════════════════════════════ */

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

/* ════════════════════════════════════
   THEME
   ════════════════════════════════════ */

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
  const sun = document.getElementById('icon-sun') as HTMLElement;
  const moon = document.getElementById('icon-moon') as HTMLElement;
  if (isDark) { root.classList.add('dark'); sun.style.display = 'none'; moon.style.display = ''; }
  else { root.classList.remove('dark'); sun.style.display = ''; moon.style.display = 'none'; }
}

/* ════════════════════════════════════
   HISTORY (localStorage)
   ════════════════════════════════════ */

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

export function addHistoryEntry(original: string, result: OptimizeOutput, language: string): void {
  const entries = loadHistory();
  entries.unshift({
    id: Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
    timestamp: Date.now(),
    original,
    optimized: result.optimized.text,
    optimizedScore: result.optimized.zipf_score,
    originalScore: result.original.zipf_score,
    language,
  });
  saveHistory(entries);
  updateHistoryBadge();
}

export function clearHistory(): void {
  localStorage.removeItem('fp_history');
  updateHistoryBadge();
  renderHistory();
  showToast('历史记录已清空');
}

export function exportHistory(): void {
  const entries = loadHistory();
  const blob = new Blob([JSON.stringify(entries, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `freqprompt-history-${new Date().toISOString().slice(0, 10)}.json`;
  a.click();
  URL.revokeObjectURL(url);
  showToast(`已导出 ${entries.length} 条记录`);
}

export function importHistory(jsonStr: string): number {
  try {
    const data = JSON.parse(jsonStr);
    if (!Array.isArray(data)) throw new Error('Not an array');
    const existing = loadHistory();
    const existingIds = new Set(existing.map((e: HistoryEntry) => e.id));
    const newEntries = data.filter((e: HistoryEntry) => e.id && e.original && !existingIds.has(e.id));
    if (newEntries.length === 0) return 0;
    const merged = [...newEntries, ...existing].slice(0, MAX_HISTORY);
    saveHistory(merged);
    updateHistoryBadge();
    return newEntries.length;
  } catch {
    throw new Error('无效的 JSON 格式');
  }
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
    const timeStr = time.toLocaleString('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
    const delta = e.optimizedScore - e.originalScore;
    const deltaStr = delta > 0.01 ? `+${delta.toFixed(2)}` : delta < -0.01 ? delta.toFixed(2) : '±0';
    const langTag = e.language === 'zh' ? '中文' : 'EN';
    return `
      <div class="history-item" data-id="${e.id}">
        <div class="history-item-time">
          <span>${timeStr}</span>
          <span style="font-size:9.5px;color:var(--text-soft);">${langTag}</span>
        </div>
        <div class="history-item-prompt" title="${escapeHtml(e.original)}">${escapeHtml(e.original.slice(0, 100))}</div>
        <div class="history-item-score">
          ${e.originalScore.toFixed(2)} → ${e.optimizedScore.toFixed(2)}
          <span style="color:${delta > 0 ? 'var(--success)' : 'var(--text-muted)'};margin-left:3px;">${deltaStr}</span>
        </div>
        <div class="history-item-actions">
          <button class="btn btn-ghost btn-xs js-hist-copy" data-text="${escapeAttr(e.optimized)}">复制结果</button>
          <button class="btn btn-ghost btn-xs js-hist-load" data-text="${escapeAttr(e.original)}">填入输入框</button>
        </div>
      </div>`;
  }).join('');

  container.querySelectorAll('.js-hist-copy').forEach(btn => {
    btn.addEventListener('click', (ev) => {
      ev.stopPropagation();
      copyText((btn as HTMLElement).dataset.text || '', btn as HTMLElement);
    });
  });
  container.querySelectorAll('.js-hist-load').forEach(btn => {
    btn.addEventListener('click', (ev) => {
      ev.stopPropagation();
      const ta = document.getElementById('prompt-input') as HTMLTextAreaElement;
      ta.value = (btn as HTMLElement).dataset.text || '';
      ta.focus();
      updateCharCount();
      toggleHistory(false);
      showToast('已填入输入框');
    });
  });
}

export function updateHistoryBadge(): void {
  const badge = document.querySelector('#btn-history .badge-dot') as HTMLElement | null;
  if (badge) {
    badge.style.display = loadHistory().length > 0 ? '' : 'none';
  }
}

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

/* ════════════════════════════════════
   TOAST
   ════════════════════════════════════ */

let _toastTimer: ReturnType<typeof setTimeout> | null = null;
export function showToast(message: string): void {
  const el = document.getElementById('toast')!;
  el.textContent = message;
  el.classList.add('show');
  if (_toastTimer) clearTimeout(_toastTimer);
  _toastTimer = setTimeout(() => el.classList.remove('show'), 2200);
}

/* ════════════════════════════════════
   COPY
   ════════════════════════════════════ */

export async function copyText(text: string, buttonEl?: HTMLElement): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    if (buttonEl) {
      const orig = buttonEl.textContent;
      buttonEl.textContent = '已复制';
      buttonEl.style.color = 'var(--success)';
      setTimeout(() => { buttonEl.textContent = orig; buttonEl.style.color = ''; }, 1800);
    }
    showToast('已复制到剪贴板');
  } catch {
    const ta = document.createElement('textarea');
    ta.value = text; ta.style.position = 'fixed'; ta.style.opacity = '0';
    document.body.appendChild(ta); ta.select(); document.execCommand('copy');
    document.body.removeChild(ta);
    showToast('已复制');
  }
}

/* ════════════════════════════════════
   ERROR / LOADING
   ════════════════════════════════════ */

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

/* ════════════════════════════════════
   CHAR COUNT
   ════════════════════════════════════ */

export function updateCharCount(): void {
  const ta = document.getElementById('prompt-input') as HTMLTextAreaElement;
  document.getElementById('char-count')!.textContent = `${ta.value.length} 字符`;
}

export interface LiveScore {
  avg: number;
  tokens: { text: string; zipf_score: number }[];
}

export function updateLiveScore(score: LiveScore | null): void {
  const bar = document.getElementById('live-score')!;
  const valEl = document.getElementById('live-score-value')!;
  const labelEl = document.getElementById('live-score-label')!;
  const barFill = document.getElementById('live-score-bar-fill')!;

  if (!score || score.tokens.length === 0) {
    bar.style.display = 'none';
    return;
  }
  bar.style.display = 'flex';
  const avg = score.avg;
  valEl.textContent = avg.toFixed(2);
  const pct = Math.max(3, Math.min((avg / 8) * 100, 100));
  barFill.style.width = `${pct}%`;

  let color = 'var(--error)';
  if (avg >= 5.0) color = 'var(--success)';
  else if (avg >= 4.0) color = 'var(--warning)';
  barFill.style.background = color;

  const lowest = [...score.tokens].sort((a, b) => a.zipf_score - b.zipf_score).slice(0, 3);
  const lowList = lowest.map(t => `${escapeHtml(t.text)} ${t.zipf_score.toFixed(1)}`).join(' · ');
  labelEl.textContent = `${score.tokens.length} 词 · 最低: ${lowList}`;
}

/* =====================================================================
   DIFF ALGORITHM (LCS-based)
   ===================================================================== */

export type DiffOp = { op: 'equal' | 'add' | 'remove'; text: string };

export function diffWords(a: string, b: string, lang: string = 'auto'): DiffOp[] {
  const useZh = lang === 'zh' || (lang === 'auto' && /[\u4e00-\u9fff]/.test(a + b));
  const tokA = useZh ? tokenizeZh(a) : tokenizeEn(a);
  const tokB = useZh ? tokenizeZh(b) : tokenizeEn(b);

  const m = tokA.length, n = tokB.length;
  if (m === 0) return tokB.map(t => ({ op: 'add' as const, text: t }));
  if (n === 0) return tokA.map(t => ({ op: 'remove' as const, text: t }));

  if (m * n > 100000) {
    return [
      { op: 'remove', text: a },
      { op: 'add', text: b },
    ];
  }

  const dp: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (tokA[i - 1] === tokB[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }
  const ops: DiffOp[] = [];
  let i = m, j = n;
  while (i > 0 && j > 0) {
    if (tokA[i - 1] === tokB[j - 1]) {
      ops.push({ op: 'equal', text: tokA[i - 1] });
      i--; j--;
    } else if (dp[i - 1][j] >= dp[i][j - 1]) {
      ops.push({ op: 'remove', text: tokA[i - 1] });
      i--;
    } else {
      ops.push({ op: 'add', text: tokB[j - 1] });
      j--;
    }
  }
  while (i > 0) { ops.push({ op: 'remove', text: tokA[i - 1] }); i--; }
  while (j > 0) { ops.push({ op: 'add', text: tokB[j - 1] }); j--; }
  ops.reverse();
  return ops;
}

function tokenizeEn(text: string): string[] {
  return text.split(/(\s+|[.,!?;:"""''()\[\]{}]|\n)/g).filter(t => t.length > 0);
}

function tokenizeZh(text: string): string[] {
  return Array.from(text);
}

export function renderDiff(ops: DiffOp[]): string {
  if (ops.length === 0) return '';
  return ops.map(op => {
    const txt = escapeHtml(op.text);
    if (op.op === 'equal') return `<span class="diff-eq">${txt}</span>`;
    if (op.op === 'add') return `<span class="diff-add">${txt}</span>`;
    return `<span class="diff-rm">${txt}</span>`;
  }).join('');
}

/* ════════════════════════════════════
   SCORE HELPERS
   ════════════════════════════════════ */

function scoreColor(score: number): string {
  if (score >= 5.0) return 'score-high';
  if (score >= 4.0) return 'score-mid';
  return 'score-low';
}

function wfColor(score: number): string {
  if (score >= 6.0) return 'wf-high';
  if (score >= 4.5) return 'wf-mid';
  return 'wf-low';
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
  const pct = Math.max(0.03, Math.min(score / max, 1));
  const r = 24;
  const circ = 2 * Math.PI * r;
  const dash = pct * circ;
  let stroke = 'var(--error)';
  if (pct >= 0.625) stroke = 'var(--success)';
  else if (pct >= 0.5) stroke = 'var(--warning)';

  return `
    <svg width="60" height="60" viewBox="0 0 60 60">
      <circle cx="30" cy="30" r="${r}" fill="none" stroke="var(--hairline)" stroke-width="4.5"/>
      <circle cx="30" cy="30" r="${r}" fill="none" stroke="${stroke}" stroke-width="4.5"
        stroke-dasharray="${dash.toFixed(1)} ${(circ - dash).toFixed(1)}"
        stroke-linecap="round" transform="rotate(-90 30 30)"
        style="transition: stroke-dasharray 0.6s cubic-bezier(0.4,0,0.2,1);"/>
      <text x="30" y="33" text-anchor="middle"
        font-family="JetBrains Mono, monospace" font-size="14" font-weight="600"
        fill="var(--text)">${score.toFixed(1)}</text>
    </svg>`;
}

/* ════════════════════════════════════
   RESULTS RENDERING
   ════════════════════════════════════ */

export function renderResults(
  result: OptimizeOutput,
  wordScores?: { original: FrequencyResult[]; optimized: FrequencyResult[] },
  roundMeta?: RoundMeta
): void {
  const empty = document.getElementById('empty-state')!;
  const content = document.getElementById('results-content')!;
  empty.style.display = 'none';
  content.style.display = 'grid';

  const delta = result.optimized.zipf_score - result.original.zipf_score;
  const optPct = Math.round((result.optimized.zipf_score / 8) * 100);
  const origPct = Math.round((result.original.zipf_score / 8) * 100);

  let html = '';

  // ── Score hero (full width) ──
  html += `
    <div class="score-hero">
      <div class="score-ring-wrap">${renderScoreRing(result.optimized.zipf_score)}</div>
      <div class="score-info">
        <div class="score-label">优化后得分 · Zipf 算术均值 (0–8)</div>
        <div class="score-value">${result.optimized.zipf_score.toFixed(2)}</div>
        <div class="score-delta ${deltaClass(delta)}">
          ${fmtDelta(delta)}
          <span style="font-size:11px;font-weight:400;color:var(--text-muted);">
            vs 原始 ${result.original.zipf_score.toFixed(2)}
            ${roundMeta?.r2Applied ? ` · ${roundMeta.beamWidth || 3} 束精修` : ''}
          </span>
        </div>
        <div class="score-bar-wrap">
          <div class="score-bar-fill orig" style="width:${origPct}%;" title="原始 ${result.original.zipf_score.toFixed(2)}"></div>
          <div class="score-bar-fill opt" style="width:${optPct - origPct}%;" title="提升 +${delta.toFixed(2)}"></div>
        </div>
      </div>
    </div>`;

  // ── Optimized prompt card (left) ──
  html += `
    <div class="prompt-card">
      <div class="prompt-card-header">
        <span class="prompt-card-label">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3l1.5 5.5L19 10l-5.5 1.5L12 17l-1.5-5.5L5 10l5.5-1.5z"/></svg>
          优化后 Prompt
        </span>
        <span class="prompt-card-badge badge-best">最佳</span>
      </div>
      <div class="prompt-card-body">${escapeHtml(result.optimized.text)}</div>
      <div class="prompt-card-actions">
        <button class="btn btn-sm js-copy" data-text="${escapeAttr(result.optimized.text)}">复制</button>
        <button class="btn btn-ghost btn-sm js-use-as-input" data-text="${escapeAttr(result.optimized.text)}">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 14 4 9 9 4"/><path d="M20 20v-7a4 4 0 0 0-4-4H4"/></svg>
          作为新输入
        </button>
      </div>
    </div>`;

  // ── Original prompt card (right) ──
  html += `
    <div class="prompt-card">
      <div class="prompt-card-header">
        <span class="prompt-card-label">原始 Prompt</span>
        <span class="prompt-card-badge badge-orig">原始</span>
      </div>
      <div class="prompt-card-body is-original">${escapeHtml(result.original.text)}</div>
      <div class="prompt-card-actions">
        <button class="btn btn-ghost btn-sm js-copy" data-text="${escapeAttr(result.original.text)}">复制</button>
      </div>
    </div>`;

  // ── Diff view (full width) ──
  const isZh = /[\u4e00-\u9fff]/.test(result.original.text + result.optimized.text);
  const diffOps = diffWords(result.original.text, result.optimized.text, isZh ? 'zh' : 'en');
  const addCount = diffOps.filter(o => o.op === 'add').length;
  const rmCount = diffOps.filter(o => o.op === 'remove').length;
  const eqCount = diffOps.filter(o => o.op === 'equal').length;
  html += `
    <div class="diff-section">
      <div class="diff-header">
        <span class="diff-label">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M8 3v18M16 3v18M3 8h3M3 16h3M18 8h3M18 16h3"/></svg>
          差异对比
        </span>
        <span class="diff-stats">
          <span class="diff-stat eq">保留 ${eqCount}</span>
          <span class="diff-stat add">新增 ${addCount}</span>
          <span class="diff-stat rm">删除 ${rmCount}</span>
        </span>
      </div>
      <div class="diff-body">${renderDiff(diffOps)}</div>
    </div>`;

  // ── Word frequency breakdown (full width) ──
  if (wordScores && wordScores.optimized.length > 0) {
    html += `
      <div class="word-freq-section">
        <div class="word-freq-header">
          <span>词汇频率明细（算术均值：${result.optimized.zipf_score.toFixed(2)}）</span>
          <span style="font-weight:400;text-transform:none;letter-spacing:0;font-size:11px;">
            共 ${wordScores.optimized.length} 个词
          </span>
        </div>
        <div class="word-freq-grid">`;

    for (const w of wordScores.optimized) {
      html += `<div class="word-freq-item">
        <span class="word-freq-word" title="${escapeHtml(w.text)}">${escapeHtml(w.text)}</span>
        <span class="word-freq-score ${wfColor(w.zipf_score)}">${w.zipf_score.toFixed(1)}</span>
      </div>`;
    }

    html += `</div></div>`;
  }

  // ── Candidates (full width) ──
  html += `<div class="candidates-section">
    <div class="candidates-header">
      <span class="candidates-label">所有候选版本</span>
      <span class="candidates-count">${result.candidates.length} 个</span>
    </div>`;

  for (let i = 0; i < result.candidates.length; i++) {
    const c = result.candidates[i];
    const isOpt = c.text === result.optimized.text;
    const isOrig = c.text === result.original.text;
    let cls = isOpt ? 'is-best' : isOrig ? 'is-original' : '';
    let tag = isOpt
      ? '<span class="tag-inline best">最佳</span>'
      : isOrig
        ? '<span class="tag-inline orig">原始</span>'
        : '';

    html += `
      <div class="candidate-row ${cls}">
        <span class="candidate-rank">#${i + 1}</span>
        <span class="candidate-text">${escapeHtml(c.text)}${tag}</span>
        <span class="candidate-score ${scoreColor(c.zipf_score)}">${c.zipf_score.toFixed(2)}</span>
      </div>`;
  }
  html += `</div>`;

  content.innerHTML = html;
  bindResultActions(result);
}

function bindResultActions(result: OptimizeOutput): void {
  document.querySelectorAll('.js-copy').forEach(btn => {
    btn.addEventListener('click', () => {
      copyText((btn as HTMLElement).dataset.text || '', btn as HTMLElement);
    });
  });
  document.querySelectorAll('.js-use-as-input').forEach(btn => {
    btn.addEventListener('click', () => {
      const ta = document.getElementById('prompt-input') as HTMLTextAreaElement;
      ta.value = (btn as HTMLElement).dataset.text || '';
      ta.focus();
      updateCharCount();
      showToast('已填入输入框');
      ta.scrollIntoView({ behavior: 'smooth', block: 'center' });
    });
  });
}

/* ════════════════════════════════════
   UTILS
   ════════════════════════════════════ */

export function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function escapeAttr(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
