/**
 * FreqPrompt v2 — Entry point
 * Event bindings, optimize flow, keyboard shortcuts.
 */
import { generateParaphrases } from './paraphrase';
import { optimizePrompt } from './frequency';
import {
  loadConfig,
  saveConfig,
  populateConfigForm,
  updateConfigIndicator,
  toggleSettings,
  initTheme,
  toggleTheme,
  toggleHistory,
  addHistoryEntry,
  updateHistoryBadge,
  showToast,
  showError,
  hideError,
  setLoading,
  updateCharCount,
  renderResults,
} from './ui';

/* ══════════════════════════════════════════
   OPTIMIZE FLOW
   ══════════════════════════════════════════ */

async function handleOptimize(): Promise<void> {
  hideError();

  const config = loadConfig();
  if (!config.baseUrl || !config.apiKey || !config.modelId) {
    toggleSettings(true);
    showError('请先配置 API 信息。');
    return;
  }

  const ta = document.getElementById('prompt-input') as HTMLTextAreaElement;
  const originalPrompt = ta.value.trim();
  if (!originalPrompt) {
    showError('请输入要优化的 Prompt。');
    return;
  }

  setLoading(true);

  try {
    const candidates = await generateParaphrases(originalPrompt, config);

    if (candidates.length === 0) {
      showError('LLM 未返回有效改写结果，请检查 API 配置或尝试其他 Prompt。');
      setLoading(false);
      return;
    }

    const result = await optimizePrompt(originalPrompt, candidates);
    renderResults(result);

    // Save to history
    addHistoryEntry(originalPrompt, result);
  } catch (err: any) {
    showError(`优化失败：${err.message || '未知错误'}`);
  } finally {
    setLoading(false);
  }
}

/* ══════════════════════════════════════════
   EVENT BINDINGS
   ══════════════════════════════════════════ */

function bindEvents(): void {
  // ── Settings modal ──
  document.getElementById('config-indicator')!.addEventListener('click', () => toggleSettings(true));
  document.getElementById('btn-cancel-settings')!.addEventListener('click', () => toggleSettings(false));

  document.getElementById('save-config')!.addEventListener('click', () => {
    const config = {
      baseUrl: (document.getElementById('base-url') as HTMLInputElement).value.trim(),
      apiKey: (document.getElementById('api-key') as HTMLInputElement).value.trim(),
      modelId: (document.getElementById('model-id') as HTMLInputElement).value.trim(),
    };
    saveConfig(config);
    updateConfigIndicator(config);
    toggleSettings(false);
    showToast('API 配置已保存');
    hideError();
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      toggleSettings(false);
      toggleHistory(false);
    }
    // ⌘, or Ctrl+, for settings
    if ((e.metaKey || e.ctrlKey) && e.key === ',') {
      e.preventDefault();
      toggleSettings(true);
    }
  });

  // ── Theme toggle ──
  document.getElementById('btn-theme')!.addEventListener('click', toggleTheme);

  // ── History drawer ──
  document.getElementById('btn-history')!.addEventListener('click', () => toggleHistory(true));
  document.getElementById('btn-close-history')!.addEventListener('click', () => toggleHistory(false));

  // ── Example chips ──
  document.querySelectorAll('.example-chip').forEach(chip => {
    chip.addEventListener('click', () => {
      const ta = document.getElementById('prompt-input') as HTMLTextAreaElement;
      ta.value = (chip as HTMLElement).textContent || '';
      ta.focus();
      updateCharCount();
    });
  });

  // ── Char count ──
  document.getElementById('prompt-input')!.addEventListener('input', updateCharCount);

  // ── Optimize ──
  document.getElementById('optimize-btn')!.addEventListener('click', handleOptimize);

  // ⌘Enter / Ctrl+Enter to optimize
  document.getElementById('prompt-input')!.addEventListener('keydown', (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      handleOptimize();
    }
    // ⌘K to clear
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      const ta = e.target as HTMLTextAreaElement;
      ta.value = '';
      updateCharCount();
      ta.focus();
    }
  });
}

/* ══════════════════════════════════════════
   INIT
   ══════════════════════════════════════════ */

document.addEventListener('DOMContentLoaded', () => {
  initTheme();
  populateConfigForm();
  updateHistoryBadge();
  updateCharCount();
  bindEvents();
});
