/**
 * LLM API client for generating paraphrases.
 * Uses OpenAI-compatible chat completions API.
 */

export interface ApiConfig {
  baseUrl: string;
  apiKey: string;
  modelId: string;
}

const PARAPHRASE_SYSTEM_PROMPT = `You are a linguistic expert specializing in rephrasing text. Your task is to rewrite the given prompt using more common, higher-frequency English words while preserving the EXACT meaning.

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

export async function generateParaphrases(
  originalPrompt: string,
  config: ApiConfig
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
        {
          role: 'system',
          content: PARAPHRASE_SYSTEM_PROMPT,
        },
        {
          role: 'user',
          content: originalPrompt,
        },
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
    // Remove any numbering prefixes like "1. " or "1) "
    .map((l: string) => l.replace(/^\d+[\.\)]\s*/, '').trim())
    .filter((l: string) => l.length > 10); // Filter out very short lines

  return lines.slice(0, 6);
}
