/**
 * FreqPrompt — Core module tests
 */
import { describe, it, expect } from 'vitest';
import { detectLanguage } from './lang';
import { cosineSimilarity } from './semantic';
import { DEFAULT_WEIGHTS } from './pipeline';

/* ═══════════ Language Detection ═══════════ */

describe('detectLanguage', () => {
  it('returns zh for Chinese text', () => {
    expect(detectLanguage('今天天气很好')).toBe('zh');
    expect(detectLanguage('请阐述财政政策调整对宏观经济均衡状态的影响机制')).toBe('zh');
    expect(detectLanguage('你好世界')).toBe('zh');
  });

  it('returns en for English text', () => {
    expect(detectLanguage('The weather is nice today')).toBe('en');
    expect(detectLanguage('Explain the fiscal policy mechanism')).toBe('en');
  });

  it('returns zh for mixed text with majority CJK', () => {
    // "今天 is a nice day" — 4/17 chars are CJK = 23.5%, below threshold
    expect(detectLanguage('Hello 世界')).toBe('en');
  });

  it('handles empty/whitespace-only text', () => {
    expect(detectLanguage('')).toBe('en');
    expect(detectLanguage('   ')).toBe('en');
  });

  it('returns zh for single CJK character', () => {
    expect(detectLanguage('的')).toBe('zh');
    expect(detectLanguage('好')).toBe('zh');
  });
});

/* ═══════════ Semantic: Cosine Similarity ═══════════ */

describe('cosineSimilarity', () => {
  it('returns 1 for identical vectors', () => {
    const vec = [0.5, 0.3, 0.2];
    expect(cosineSimilarity(vec, vec)).toBeCloseTo(1.0, 10);
  });

  it('returns 0 for orthogonal vectors', () => {
    expect(cosineSimilarity([1, 0, 0], [0, 1, 0])).toBeCloseTo(0, 10);
  });

  it('returns -1 for opposite vectors', () => {
    expect(cosineSimilarity([1, 0], [-1, 0])).toBeCloseTo(-1, 10);
  });

  it('returns 0 for zero vector', () => {
    expect(cosineSimilarity([0, 0, 0], [1, 2, 3])).toBe(0);
    expect(cosineSimilarity([1, 2, 3], [0, 0, 0])).toBe(0);
  });

  it('is symmetric', () => {
    const a = [0.1, 0.5, 0.3];
    const b = [0.7, 0.2, 0.1];
    expect(cosineSimilarity(a, b)).toBe(cosineSimilarity(b, a));
  });

  it('handles single-element vectors', () => {
    expect(cosineSimilarity([3], [3])).toBeCloseTo(1, 10);
    expect(cosineSimilarity([1], [-5])).toBeCloseTo(-1, 10);
  });
});

/* ═══════════ Pipeline: Weights ═══════════ */

describe('DEFAULT_WEIGHTS', () => {
  it('sums to 1', () => {
    const sum = Object.values(DEFAULT_WEIGHTS).reduce((a, b) => a + b, 0);
    expect(sum).toBeCloseTo(1.0, 10);
  });

  it('frequency has the highest weight', () => {
    const weights = Object.values(DEFAULT_WEIGHTS);
    expect(DEFAULT_WEIGHTS.frequency).toBe(Math.max(...weights));
  });

  it('all weights are non-negative', () => {
    for (const w of Object.values(DEFAULT_WEIGHTS)) {
      expect(w).toBeGreaterThanOrEqual(0);
    }
  });

  it('has exactly 5 layers', () => {
    expect(Object.keys(DEFAULT_WEIGHTS)).toHaveLength(5);
    expect(Object.keys(DEFAULT_WEIGHTS)).toContain('frequency');
    expect(Object.keys(DEFAULT_WEIGHTS)).toContain('collocation');
    expect(Object.keys(DEFAULT_WEIGHTS)).toContain('complexity');
    expect(Object.keys(DEFAULT_WEIGHTS)).toContain('slot_preservation');
    expect(Object.keys(DEFAULT_WEIGHTS)).toContain('domain_relevance');
  });
});
