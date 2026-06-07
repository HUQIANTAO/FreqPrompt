/**
 * FreqPrompt v2 — LLM API client for multi-round prompt optimization.
 *
 * Round 1: broad diverse rewrites (6 versions).
 * Round 2: targeted word replacement — LLM is told exactly which words
 *          scored low and is asked to replace only those words.
 */

import { detectLanguage } from './lang';

export interface ApiConfig {
  baseUrl: string;
  apiKey: string;
  modelId: string;
}

/* ═══════════════ ROUND 1: Broad Rewrites ═══════════════ */

const R1_SYSTEM_EN = `You are a linguistic expert. Rewrite the given text using more common, higher-frequency words while preserving the EXACT meaning.

## Rules:
1. Replace uncommon/complex words with more frequent, everyday synonyms
2. Keep the SAME semantic content — do not add, remove, or change meaning
3. Maintain the same sentence structure as much as possible
4. Generate exactly 6 different versions, each on its own line
5. Vary the "formality level" from very casual to slightly formal

## Why this matters:
Language models process high-frequency words faster and more accurately. Replacing rare words with common synonyms improves LLM output quality without changing what you're asking.

Now rewrite into 6 higher-frequency versions (output ONLY the 6 lines):`;

const R1_SYSTEM_ZH = `你是一位中文语言学专家。请用更高频、更常见的汉语词汇重写给定文本，严格保持原意。

## 替换策略：
1. 书面语 → 口语：是否→是不是/吗、进行→做、该（问题）→这个（问题）、上述→上面说的、其→它的/他的、此→这、如何→怎么、为何→为什么、均→都、仅→只、若→如果
2. 学术词 → 日常词：阐述→说明/解释、呈现→表现出、具备→有、实施→执行/做、优化→改进、利用→用、导致→造成、以及→和、因此→所以、然而→但是
3. 压缩冗余：删除"在……方面""对于……来说"等虚词结构

## 版本多样性：
- 版本 1：最口语，像聊天
- 版本 2-3：通俗易懂，新闻/科普风格
- 版本 4-5：稍正式但依然常见
- 版本 6：保留专业感但不生僻

## 输出格式：每行一个版本，共 6 行，不要编号或解释。

现在请重写以下文本，生成 6 个高频词汇版本：`;

/* ═══════════════ ROUND 2: Targeted Word Replacement ═══════════════ */

const R2_SYSTEM_EN = `You are a linguistic expert. Your task is to replace SPECIFIC low-frequency words in a text with higher-frequency synonyms.

## What to do:
Below is a sentence and a list of words that scored LOW on the Zipf frequency scale (meaning they're rare/uncommon). Your job is to rewrite the sentence by replacing ONLY those low-frequency words with more common alternatives. Keep everything else unchanged.

## Rules:
1. ONLY change the listed low-frequency words — leave the rest of the sentence intact
2. For each low-frequency word, find the most common synonym that preserves meaning
3. If a low-frequency word has no good common synonym, try rephrasing just that phrase
4. Generate exactly 4 versions, each on its own line
5. Each version should use different synonym choices

Output ONLY the 4 rewritten sentences, one per line. No numbering or explanation.`;

const R2_SYSTEM_ZH = `你是一位中文词汇频率专家。你的任务是**只替换**句子中频率较低的词，用更常见的近义词替代。

## 任务说明
下面会给你一个句子，以及其中词频得分较低的词（分数越低 = 越生僻）。请重写句子，**只替换这些低频词**，其他部分保持不变。

## 规则：
1. 只改列出的低频词——句子其余部分保持原样
2. 每个低频词找一个最常用的近义词替代
3. 如果一个词没有好的近义词，试着改写那个短语
4. 生成 4 个版本，每行一个
5. 每个版本用不同的近义词选择

## 替换示例：
阐述(低分)→说明/解释/讲清楚  ·  实施(低分)→执行/落实/做  ·  该(低分)→这个  ·  上述(低分)→上面说的  ·  其(低分)→它的/他的  ·  促进(低分)→推动/帮助  ·  路径(低分)→方法/方式/做法

只输出 4 行改写结果，不要编号或解释。`;

/* ═══════════════ API Client ═══════════════ */

async function callLLM(
  systemPrompt: string,
  userMessage: string,
  config: ApiConfig,
  temperature: number = 0.0
): Promise<string[]> {
  const url = `${config.baseUrl.replace(/\/$/, '')}/chat/completions`;

  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${config.apiKey}`,
    },
    body: JSON.stringify({
      model: config.modelId,
      messages: [
        { role: 'system', content: systemPrompt },
        { role: 'user', content: userMessage },
      ],
      temperature,
    }),
  });

  if (!response.ok) {
    const errorBody = await response.text();
    throw new Error(`API error ${response.status}: ${errorBody.slice(0, 300)}`);
  }

  const data = await response.json();
  const content = data.choices?.[0]?.message?.content;
  if (!content) throw new Error('API returned empty response');

  return content
    .split('\n')
    .map((l: string) => l.trim())
    .filter((l: string) => l.length > 0)
    .map((l: string) => l.replace(/^\d+[\.\)、]\s*/, '').trim())
    .filter((l: string) => l.length > 5);
}

/** Round 1: Generate 6 broad, diverse paraphrases. */
export async function generateParaphrases(
  originalPrompt: string,
  config: ApiConfig
): Promise<string[]> {
  const zh = detectLanguage(originalPrompt) === 'zh';
  const system = zh ? R1_SYSTEM_ZH : R1_SYSTEM_EN;
  const lines = await callLLM(system, originalPrompt, config, 0.8);
  return lines.slice(0, 6);
}

/**
 * Round 2: Targeted replacement of specific low-frequency words.
 *
 * @param currentBest — the best result from Round 1
 * @param lowWords — list of {word, score} pairs that scored lowest
 * @param config — API configuration
 * @returns up to 4 targeted rewrites
 */
export async function targetedReplace(
  currentBest: string,
  lowWords: { text: string; zipf_score: number }[],
  config: ApiConfig
): Promise<string[]> {
  const zh = detectLanguage(currentBest) === 'zh';
  const system = zh ? R2_SYSTEM_ZH : R2_SYSTEM_EN;

  // Build a clear list of words to replace
  const wordList = lowWords
    .map(w => `  - "${w.text}" (频率分: ${w.zipf_score.toFixed(1)})`)
    .join('\n');

  const userMessage = zh
    ? `原始句子：\n${currentBest}\n\n需要替换的低频词（分数越低越需要换）：\n${wordList}\n\n请只替换这些词，生成 4 个版本：`
    : `Original sentence:\n${currentBest}\n\nLow-frequency words to replace (lower = rarer):\n${wordList}\n\nReplace only these words, generate 4 versions:`;

  const lines = await callLLM(system, userMessage, config, 0.6);
  return lines.slice(0, 4);
}
