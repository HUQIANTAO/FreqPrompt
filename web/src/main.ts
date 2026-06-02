/**
 * FreqPrompt v2 — Entry point
 *
 * Beam-search iterative optimization:
 *   Round 1: LLM generates 6 broad paraphrases → WASM scores → keep top-3 (beam)
 *   Round 2: For each of the 3 beams, do targeted low-frequency word replacement
 *            → 4 versions per beam → 12 candidates
 *   Final:   Score all beam outputs together, pick overall best.
 *
 * Real-time scoring: as the user types, debounced WASM scoring shows
 * the current prompt's score and color-coded per-word breakdown.
 */
import { generateParaphrases, targetedReplace } from './paraphrase';
import { optimizePrompt, scoreSentences, tokenizeAndScore, lowestTokens } from './frequency';
import {
  loadConfig, saveConfig, populateConfigForm, updateConfigIndicator,
  toggleSettings, initTheme, toggleTheme,
  toggleHistory, addHistoryEntry, updateHistoryBadge, renderHistory,
  clearHistory, exportHistory, importHistory,
  showToast, showError, hideError, setLoading, updateCharCount,
  renderResults, showRoundProgress, updateLiveScore,
} from './ui';

/* ════════════════════════════════════
   LANGUAGE DETECTION
   ════════════════════════════════════ */

function detectLanguage(text: string): string {
  let cjk = 0, total = 0;
  for (const c of text) {
    if (c.trim() === '') continue;
    total++;
    const code = c.codePointAt(0)!;
    if ((code >= 0x4E00 && code <= 0x9FFF) ||
        (code >= 0x3400 && code <= 0x4DBF) ||
        (code >= 0x3000 && code <= 0x303F)) {
      cjk++;
    }
  }
  return total > 0 && cjk / total > 0.3 ? 'zh' : 'en';
}

/* ════════════════════════════════════
   BEAM-SEARCH OPTIMIZE FLOW
   ════════════════════════════════════ */

const BEAM_WIDTH = 3;
const ROUND2_PER_BEAM = 4;

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
  const lang = detectLanguage(originalPrompt);
  const roundLabel = lang === 'zh' ? '轮' : 'round';
  const beamLabel = lang === 'zh' ? '束' : 'beam';

  try {
    /* ── Round 1: Broad paraphrases → score → keep top BEAM_WIDTH ── */
    showRoundProgress(`第 1 ${roundLabel}：生成多样化改写（束宽 ${BEAM_WIDTH}）…`);
    const r1Candidates = await generateParaphrases(originalPrompt, config);

    if (r1Candidates.length === 0) {
      showError('LLM 未返回有效改写结果，请检查 API 配置或尝试其他 Prompt。');
      setLoading(false);
      return;
    }

    // Score everything (R1 candidates + original) once
    const r1AllText = [...new Set([originalPrompt, ...r1Candidates])];
    const r1Scored = await scoreSentences(r1AllText);

    // Sort by score desc, keep top BEAM_WIDTH (excluding original which is the baseline)
    const beams = r1Scored
      .filter(r => r.text !== originalPrompt)
      .slice(0, BEAM_WIDTH);

    if (beams.length === 0) {
      showError('LLM 未返回有效改写结果。');
      setLoading(false);
      return;
    }

    const r1Best = beams[0];
    const originalScore = r1Scored.find(r => r.text === originalPrompt)!.zipf_score;
    const r1Delta = r1Best.zipf_score - originalScore;

    /* ── Round 2: Per-beam targeted replacement ── */
    let r2Applied = false;
    let r2Candidates: string[] = [];

    if (r1Delta < 1.5) {
      showRoundProgress(`第 2 ${roundLabel}：${BEAM_WIDTH} 个束各自精修低频词…`);

      // Process beams in parallel — each beam gets its own targeted replacement
      const r2Results = await Promise.allSettled(
        beams.map(async (beam) => {
          const lowWords = await lowestTokens(beam.text, lang === 'zh' ? 8 : 6);
          if (lowWords.length < 2) return [];
          return targetedReplace(beam.text, lowWords, config);
        })
      );

      for (const r of r2Results) {
        if (r.status === 'fulfilled') {
          r2Candidates.push(...r.value);
        }
      }
      r2Candidates = [...new Set(r2Candidates)].filter(c => c !== originalPrompt);
    }

    /* ── Final scoring: combine all candidates + beams + original ── */
    const allText = [...new Set([
      originalPrompt,
      ...beams.map(b => b.text),
      ...r2Candidates,
    ])];
    const finalScored = await scoreSentences(allText);
    const finalBest = finalScored[0];

    if (r2Candidates.length > 0 && finalBest.zipf_score > r1Best.zipf_score + 0.02) {
      r2Applied = true;
    }

    // Get per-word scores for the frequency breakdown
    let wordScores: { original: { text: string; zipf_score: number }[]; optimized: { text: string; zipf_score: number }[] } | undefined;
    try {
      const [origTokens, optTokens] = await Promise.all([
        tokenizeAndScore(originalPrompt),
        tokenizeAndScore(finalBest.text),
      ]);
      wordScores = { original: origTokens, optimized: optTokens };
    } catch { /* word scoring is optional */ }

    const finalResult = {
      original: { text: originalPrompt, zipf_score: originalScore },
      optimized: finalBest,
      candidates: finalScored,
    };

    renderResults(finalResult, wordScores, { r1Delta, r2Applied, beamWidth: BEAM_WIDTH });
    addHistoryEntry(originalPrompt, finalResult, lang);

    const totalDelta = finalBest.zipf_score - originalScore;
    const msg = lang === 'zh'
      ? `优化完成！${r2Applied ? `${BEAM_WIDTH} 束精修` : ''} 提升 +${totalDelta.toFixed(2)}`
      : `Optimized!${r2Applied ? ` ${BEAM_WIDTH}-beam` : ''} +${totalDelta.toFixed(2)}`;
    showToast(msg);
  } catch (err: any) {
    showError(`优化失败：${err.message || '未知错误'}`);
  } finally {
    setLoading(false);
    showRoundProgress('');
  }
}

/* ════════════════════════════════════
   REAL-TIME SCORING (debounced)
   ════════════════════════════════════ */

let liveScoreTimer: number | null = null;

async function handleLiveScore(): Promise<void> {
  const ta = document.getElementById('prompt-input') as HTMLTextAreaElement;
  const text = ta.value.trim();
  if (!text) {
    updateLiveScore(null);
    return;
  }
  try {
    const tokens = await tokenizeAndScore(text);
    // Compute average score
    const avg = tokens.length > 0
      ? tokens.reduce((s, t) => s + t.zipf_score, 0) / tokens.length
      : 0;
    updateLiveScore({ avg, tokens });
  } catch (err) {
    // silent — live scoring is best-effort
  }
}

function scheduleLiveScore(): void {
  if (liveScoreTimer !== null) {
    clearTimeout(liveScoreTimer);
  }
  liveScoreTimer = window.setTimeout(handleLiveScore, 300);
}

/* ════════════════════════════════════
   EVENT BINDINGS
   ════════════════════════════════════ */

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
    if ((e.metaKey || e.ctrlKey) && e.key === ',') {
      e.preventDefault();
      toggleSettings(true);
    }
  });

  // ── Theme ──
  document.getElementById('btn-theme')!.addEventListener('click', toggleTheme);

  // ── History drawer ──
  document.getElementById('btn-history')!.addEventListener('click', () => toggleHistory(true));
  document.getElementById('btn-close-history')!.addEventListener('click', () => toggleHistory(false));

  document.getElementById('btn-export-history')!.addEventListener('click', exportHistory);
  document.getElementById('btn-import-history')!.addEventListener('click', () => {
    document.getElementById('import-file-input')!.click();
  });
  document.getElementById('import-file-input')!.addEventListener('change', (e) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const count = importHistory(reader.result as string);
        showToast(count > 0 ? `已导入 ${count} 条记录` : '没有新记录可导入');
      } catch (err: any) {
        showToast(`导入失败：${err.message}`);
      }
      (e.target as HTMLInputElement).value = '';
    };
    reader.readAsText(file);
  });

  document.getElementById('btn-clear-history')!.addEventListener('click', () => {
    if (confirm('确定要清空所有历史记录吗？此操作不可撤销。')) clearHistory();
  });

  // ── Example chips ──
  document.querySelectorAll('.example-chip').forEach(chip => {
    chip.addEventListener('click', () => {
      const ta = document.getElementById('prompt-input') as HTMLTextAreaElement;
      ta.value = (chip as HTMLElement).textContent || '';
      ta.focus();
      updateCharCount();
      scheduleLiveScore();
    });
  });

  // ── Char count + live score ──
  document.getElementById('prompt-input')!.addEventListener('input', () => {
    updateCharCount();
    scheduleLiveScore();
  });

  // ── Optimize ──
  document.getElementById('optimize-btn')!.addEventListener('click', handleOptimize);

  document.getElementById('prompt-input')!.addEventListener('keydown', (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      handleOptimize();
    }
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      (e.target as HTMLTextAreaElement).value = '';
      updateCharCount();
      updateLiveScore(null);
      (e.target as HTMLTextAreaElement).focus();
    }
  });
}

/* ════════════════════════════════════
   INIT
   ════════════════════════════════════ */

document.addEventListener('DOMContentLoaded', () => {
  initTheme();
  populateConfigForm();
  updateHistoryBadge();
  updateCharCount();
  bindEvents();
});
