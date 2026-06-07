/**
 * FreqPrompt v2/v3 — Entry point
 *
 * Beam-search iterative optimization:
 *   Round 1: LLM generates 6 broad paraphrases → WASM scores → keep top-3 (beam)
 *   Round 2: For each of the 3 beams, do targeted low-frequency word replacement
 *            → 4 versions per beam → 12 candidates
 *   Final:   Score all beam outputs together, pick overall best.
 *
 * v3 Semantic Guard: between R1 and final scoring, candidates are verified
 * against the original prompt's semantic slots (Action, Object, Scope, Qtype,
 * Register, Placeholder, Negation, Condition). Candidates that drop or
 * semantically shift critical slots are blocked. This prevents the
 * "财政政策 → 国税规定" / "影响机制 → 过程" failure modes.
 *
 * v3 Sprint 2: Ontology Guard — prevents parent↔child substitution
 * v3 Sprint 3: Multi-layer scoring (frequency + collocation + complexity + slot + domain)
 * v3 Sprint 4: Domain adaptation (custom frequency tables from user corpus)
 *
 * Real-time scoring: as the user types, debounced WASM scoring shows
 * the current prompt's score and color-coded per-word breakdown.
 */
import { detectLanguage } from './lang';
import { generateParaphrases, targetedReplace } from './paraphrase';
import { optimizePrompt, scoreSentences, tokenizeAndScore, lowestTokens } from './frequency';
import {
  HeuristicEmbedder,
  filterPreservingCandidates,
  extractSlots,
  type Slot,
  type VerifyOutput,
} from './semantic';
import {
  loadOntology,
  detectDomain,
  batchCheckSubstitutions,
  type SubstitutionResult,
} from './ontology';
import {
  computeMultiLayerScore,
  renderWeightConfig,
} from './pipeline';
import {
  loadConfig, saveConfig, populateConfigForm, updateConfigIndicator,
  toggleSettings, initTheme, toggleTheme,
  toggleHistory, addHistoryEntry, updateHistoryBadge, renderHistory,
  clearHistory, exportHistory, importHistory,
  showToast, showError, hideError, setLoading, updateCharCount,
  showConfirm,
  renderResults, showRoundProgress, updateLiveScore,
  renderSlotGuardReport, renderOntologyReport, renderMultiLayerReport,
  renderDetectedDomain,
} from './ui';

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

  // v3 Semantic Guard state
  const embedder = new HeuristicEmbedder();
  let originalSlots: Slot[] = [];
  let semanticFiltered: { total: number; kept: number; dropped: VerifyOutput[] } | null = null;
  let ontologyFiltered: SubstitutionResult[] = [];
  let detectedDomain: string | null = null;

  try {
    /* ── Round 0: Extract semantic slots + detect domain ── */
    showRoundProgress(`第 0 ${roundLabel}：分析语义槽位…`);
    try {
      [originalSlots] = await Promise.all([
        extractSlots(originalPrompt, lang === 'zh' ? 'zh' : 'en'),
        detectDomain(originalPrompt, lang).then(d => {
          detectedDomain = d?.domain ?? null;
        }).catch(() => {}),
      ]);
    } catch {
      originalSlots = [];
    }

    /* ── Round 1: Broad paraphrases → score → keep top BEAM_WIDTH ── */
    showRoundProgress(`第 1 ${roundLabel}：生成多样化改写（束宽 ${BEAM_WIDTH}）…`);
    const r1Candidates = await generateParaphrases(originalPrompt, config);

    if (r1Candidates.length === 0) {
      showError('LLM 未返回有效改写结果，请检查 API 配置或尝试其他 Prompt。');
      setLoading(false);
      return;
    }

    /* ── v3 Semantic Guard: filter R1 candidates ── */
    let r1Survivors: string[] = r1Candidates;
    if (originalSlots.length > 0) {
      showRoundProgress(`第 1 ${roundLabel}：语义守门检查 ${r1Candidates.length} 个候选…`);
      const verified = await filterPreservingCandidates(originalPrompt, r1Candidates, embedder);
      const dropped = verified.filter(v => !v.passes);
      r1Survivors = verified.filter(v => v.passes).map(v => v.text);

      semanticFiltered = {
        total: verified.length,
        kept: r1Survivors.length,
        dropped: dropped.map(d => d.output),
      };

      // If filtering removed everything, log and fall back to using the most-
      // semantically-faithful candidate rather than failing outright.
      if (r1Survivors.length === 0 && verified.length > 0) {
        r1Survivors = [verified.sort((a, b) => b.score - a.score)[0].text];
        if (semanticFiltered) semanticFiltered.kept = 1;
      }
    }

    /* ── v3 Sprint 2: Ontology Guard — block parent→child substitutions ── */
    if (r1Survivors.length > 0 && detectedDomain) {
      try {
        const ontologyJson = await loadOntology(detectedDomain, lang);
        if (ontologyJson !== '{}') {
          const pairs: [string, string][] = r1Survivors.map(c => [originalPrompt, c] as [string, string]);
          const ontologyResults = await batchCheckSubstitutions(pairs, ontologyJson);
          const unsafeResults = ontologyResults.filter(r => !r.is_safe);
          if (unsafeResults.length > 0) {
            ontologyFiltered = unsafeResults;
            // Remove unsafe candidates
            const unsafeTexts = new Set(unsafeResults.map(r => r.candidate));
            r1Survivors = r1Survivors.filter(c => !unsafeTexts.has(c));
          }
        }
      } catch {
        // ontology check is optional
      }
    }

    // Score everything (R1 survivors + original) once
    const r1AllText = [...new Set([originalPrompt, ...r1Survivors])];
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

      // v3 Semantic Guard: filter R2 candidates too
      if (r2Candidates.length > 0 && originalSlots.length > 0) {
        const r2Verified = await filterPreservingCandidates(originalPrompt, r2Candidates, embedder);
        const r2Dropped = r2Verified.filter(v => !v.passes);
        r2Candidates = r2Verified.filter(v => v.passes).map(v => v.text);

        // Accumulate drops into the report
        if (semanticFiltered) {
          semanticFiltered.total += r2Verified.length;
          semanticFiltered.kept += r2Candidates.length;
          semanticFiltered.dropped.push(...r2Dropped.map(d => d.output));
        }
        if (r2Candidates.length === 0 && r2Verified.length > 0) {
          r2Candidates = [r2Verified.sort((a, b) => b.score - a.score)[0].text];
          if (semanticFiltered) semanticFiltered.kept = (semanticFiltered.kept || 0) + 1;
        }
      }

      // v3 Sprint 2: Ontology Guard for R2 candidates
      if (r2Candidates.length > 0 && detectedDomain) {
        try {
          const ontologyJson = await loadOntology(detectedDomain, lang);
          if (ontologyJson !== '{}') {
            const pairs: [string, string][] = r2Candidates.map(c => [originalPrompt, c] as [string, string]);
            const ontologyResults = await batchCheckSubstitutions(pairs, ontologyJson);
            const unsafeResults = ontologyResults.filter(r => !r.is_safe);
            if (unsafeResults.length > 0) {
              ontologyFiltered.push(...unsafeResults);
              const unsafeTexts = new Set(unsafeResults.map(r => r.candidate));
              r2Candidates = r2Candidates.filter(c => !unsafeTexts.has(c));
            }
          }
        } catch {
          // ontology check is optional
        }
      }
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

    // v3: always show semantic guard report
    if (semanticFiltered) {
      renderSlotGuardReport(semanticFiltered, originalSlots, lang);
    }

    // v3 Sprint 2: always show ontology guard report
    renderOntologyReport(ontologyFiltered, lang);

    // v3 Sprint 3: always show multi-layer score
    try {
      const slotPres = semanticFiltered
        ? semanticFiltered.kept / Math.max(semanticFiltered.total, 1)
        : 1.0;
      const mlScores = await computeMultiLayerScore(finalBest.text, slotPres);
      renderMultiLayerReport(mlScores, lang);
    } catch (e) {
      console.warn('Multi-layer score failed:', e);
    }

    const totalDelta = finalBest.zipf_score - originalScore;
    let msg: string;
    if (lang === 'zh') {
      const dropped = semanticFiltered?.dropped.length ?? 0;
      const ontologyBlocked = ontologyFiltered.length;
      msg = `优化完成！${r2Applied ? `${BEAM_WIDTH} 束精修 ` : ''}提升 +${totalDelta.toFixed(2)}`
        + (dropped > 0 ? ` · 语义守门拦截 ${dropped} 条` : '')
        + (ontologyBlocked > 0 ? ` · 本体守门拦截 ${ontologyBlocked} 条` : '')
        + (detectedDomain ? ` · 领域: ${detectedDomain}` : '');
    } else {
      const dropped = semanticFiltered?.dropped.length ?? 0;
      const ontologyBlocked = ontologyFiltered.length;
      msg = `Optimized!${r2Applied ? ` ${BEAM_WIDTH}-beam ` : ''}+${totalDelta.toFixed(2)}`
        + (dropped > 0 ? ` · semantic guard blocked ${dropped}` : '')
        + (ontologyBlocked > 0 ? ` · ontology guard blocked ${ontologyBlocked}` : '')
        + (detectedDomain ? ` · domain: ${detectedDomain}` : '');
    }
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
let liveScoreGeneration = 0;

async function handleLiveScore(): Promise<void> {
  const ta = document.getElementById('prompt-input') as HTMLTextAreaElement;
  const text = ta.value.trim();
  if (!text) {
    updateLiveScore(null);
    return;
  }
  const gen = ++liveScoreGeneration;
  try {
    const tokens = await tokenizeAndScore(text);
    // Discard stale results from superseded requests
    if (gen !== liveScoreGeneration) return;
    // Compute average score
    const avg = tokens.length > 0
      ? tokens.reduce((s, t) => s + t.zipf_score, 0) / tokens.length
      : 0;
    if (gen === liveScoreGeneration) {
      updateLiveScore({ avg, tokens });
    }
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
    const baseUrl = (document.getElementById('base-url') as HTMLInputElement).value.trim();
    const apiKey = (document.getElementById('api-key') as HTMLInputElement).value.trim();
    const modelId = (document.getElementById('model-id') as HTMLInputElement).value.trim();

    // Validation
    if (!baseUrl) { showError('请输入 API 地址。'); return; }
    try { new URL(baseUrl); } catch { showError('API 地址格式不正确（需要完整的 http(s):// URL）。'); return; }
    if (!apiKey) { showError('请输入 API 密钥。'); return; }
    if (!modelId) { showError('请输入模型 ID。'); return; }

    const config = { baseUrl, apiKey, modelId };
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

  document.getElementById('btn-clear-history')!.addEventListener('click', async () => {
    const ok = await showConfirm('确定要清空所有历史记录吗？此操作不可撤销。');
    if (ok) clearHistory();
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

  // ── v3: Weight settings modal ──
  const weightModal = document.getElementById('weight-modal-overlay');
  const btnV3 = document.getElementById('btn-v3-settings');
  const btnCloseModal = document.getElementById('btn-close-weight-modal');

  if (btnV3 && weightModal) {
    btnV3.addEventListener('click', () => {
      weightModal.classList.add('open');
      // Render weight config into the modal body (only once)
      const weightCfg = weightModal.querySelector('#weight-config');
      if (weightCfg && (weightCfg as HTMLElement).children.length === 0) {
        renderWeightConfig(weightCfg as HTMLElement, (_w) => {
          // weights are persisted by renderWeightConfig's own saveWeights call
        });
      }
    });
  }
  if (btnCloseModal && weightModal) {
    btnCloseModal.addEventListener('click', () => {
      weightModal.classList.remove('open');
    });
    weightModal.addEventListener('click', (e) => {
      if (e.target === weightModal) {
        weightModal.classList.remove('open');
      }
    });
  }
});
