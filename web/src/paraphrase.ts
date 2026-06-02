/**
 * LLM API client for generating paraphrases.
 * Uses OpenAI-compatible chat completions API.
 * Auto-detects language and uses appropriate system prompt.
 */

export interface ApiConfig {
  baseUrl: string;
  apiKey: string;
  modelId: string;
}

const PARAPHRASE_SYSTEM_EN = `You are a linguistic expert specializing in rephrasing text. Your task is to rewrite the given prompt using more common, higher-frequency English words while preserving the EXACT meaning.

## Rules:
1. Replace uncommon/complex words with their more frequent synonyms
2. Keep the SAME meaning — do not add, remove, or change any semantic content
3. Maintain the same sentence structure as much as possible
4. Output ONLY the rewritten sentences, one per line, without numbering or prefixes
5. Generate exactly 6 different versions, each on its own line

## Example:
Original: "Elucidate the ramifications of the fiscal policy alterations upon macroeconomic equilibrium."
Output:
Explain the effects of the tax policy changes on the overall economy.
Describe how changes in government spending affect the economic balance.
Show the impact of financial policy shifts on the broader economy.
Tell me the results of changing money policies on the whole economic system.
Break down what spending policy changes do to the economy as a whole.
What happens to the economy when the government changes its spending policies?

Now rewrite the following prompt into 6 higher-frequency versions. Output ONLY the 6 rewritten sentences, one per line:`;

const PARAPHRASE_SYSTEM_ZH = `你是一位精通中文表达的语言学专家。你的任务是用更高频、更常见的汉语词汇重新表达给定的提示词，同时完全保留语义。

## 规则：
1. 将生僻、书面化的词汇替换为更常见、更口语化的近义词
2. 保持完全相同的语义——不得添加、删除或改变任何语义内容
3. 尽量保持相同的句式结构
4. 只输出改写后的句子，每行一个，不要编号或前缀
5. 生成恰好 6 个不同版本，每行一个
6. 使用现代标准汉语，避免文言文或过于口语化的表达

## 示例：
原文："请阐述财政政策调整对宏观经济均衡状态的影响机制。"
输出：
请说明财政政策变化如何影响宏观经济的平衡。
请解释政府调整财政政策对整体经济稳定有什么作用。
请分析改变财政政策会给经济平衡带来什么影响。
请描述财政政策变动对宏观经济会产生哪些作用。
请谈谈调整财政政策是怎么影响整个经济体系的。
请讲讲政府改变财政政策会怎样影响经济的平衡状态。

现在请将以下提示词改写成 6 个更高频词汇的版本。只输出 6 个改写后的句子，每行一个：`;

/** Detect if text is primarily Chinese (CJK characters) */
function isChinese(text: string): boolean {
  let cjk = 0;
  let total = 0;
  for (const c of text) {
    if (c.trim() === '') continue;
    total++;
    const code = c.codePointAt(0)!;
    if (
      (code >= 0x4E00 && code <= 0x9FFF) || // CJK Unified
      (code >= 0x3400 && code <= 0x4DBF) || // CJK Extension A
      (code >= 0x3000 && code <= 0x303F)    // CJK Punctuation
    ) {
      cjk++;
    }
  }
  return total > 0 && cjk / total > 0.3;
}

export async function generateParaphrases(
  originalPrompt: string,
  config: ApiConfig
): Promise<string[]> {
  const url = `${config.baseUrl.replace(/\/$/, '')}/chat/completions`;
  const chinese = isChinese(originalPrompt);
  const systemPrompt = chinese ? PARAPHRASE_SYSTEM_ZH : PARAPHRASE_SYSTEM_EN;

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
        { role: 'user', content: originalPrompt },
      ],
      temperature: 0.8,
      max_tokens: 2000,
    }),
  });

  if (!response.ok) {
    const errorBody = await response.text();
    throw new Error(`API error ${response.status}: ${errorBody.slice(0, 300)}`);
  }

  const data = await response.json();
  const content = data.choices?.[0]?.message?.content;

  if (!content) {
    throw new Error('API returned empty response');
  }

  // Parse the response: each line is a paraphrase
  const lines = content
    .split('\n')
    .map((l: string) => l.trim())
    .filter((l: string) => l.length > 0)
    .map((l: string) => l.replace(/^\d+[\.\)、]\s*/, '').trim())
    .filter((l: string) => l.length > 5);

  return lines.slice(0, 6);
}
