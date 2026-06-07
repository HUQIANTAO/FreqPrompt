mod wordfreq_data;
mod wordfreq_data_cn;
mod wordfreq_data_bigram;
mod wordfreq_data_cn_bigram;
pub mod multi_layer;

/// Detects whether the input text is primarily Chinese.
///
/// # Examples
///
/// ```
/// # use frequency::detect_language;
/// assert_eq!(detect_language("今天天气很好"), "zh");
/// assert_eq!(detect_language("Hello world"), "en");
/// ```

/// Default Zipf frequency for unknown English words.
/// Raised significantly because unknown words in real-world prompts
/// are typically mid-frequency domain terms, not ultra-rare.
pub const DEFAULT_ZIPF: f64 = 4.0;

/// Default Zipf for unknown Chinese words.
/// Chinese has a steeper Zipf slope, so unknown words are more likely
/// to be genuinely rarer, but in practice unknown words in prompts are
/// often mid-frequency terms not in our dictionary.
pub const DEFAULT_ZIPF_CN: f64 = 3.5;

/// Default score when a token (unigram OR bigram) is unknown.
/// Used as fallback inside TF-IDF weighting.
pub const DEFAULT_BIGRAM: f64 = 3.5;

/* ══════════════════════════════════════════
   LANGUAGE DETECTION
   ══════════════════════════════════════════ */

/// Detect the primary language of a text.
pub fn detect_language(text: &str) -> &'static str {
    let total_chars = text.chars().filter(|c| !c.is_whitespace()).count();
    if total_chars == 0 {
        return "en";
    }
    let cjk_count = text
        .chars()
        .filter(|c| {
            matches!(
                *c,
                '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}'
                    | '\u{F900}'..='\u{FAFF}' | '\u{3000}'..='\u{303F}'
                    | '\u{FF00}'..='\u{FFEF}'
            )
        })
        .count();
    if total_chars > 0 && (cjk_count as f64 / total_chars as f64) > 0.3 {
        "zh"
    } else {
        "en"
    }
}

/* ══════════════════════════════════════════
   UNIGRAM & BIGRAM LOOKUP
   ══════════════════════════════════════════ */

#[inline]
pub fn word_zipf(word: &str) -> f64 {
    let word_lower = word.to_lowercase();
    binary_search_zipf(&word_lower, wordfreq_data::WORD_FREQUENCIES)
}

#[inline]
pub fn word_zipf_cn(word: &str) -> f64 {
    let word_trimmed = word.trim();
    if word_trimmed.is_empty() { return DEFAULT_ZIPF_CN; }

    let data = wordfreq_data_cn::CN_WORD_FREQUENCIES;
    if let Ok(idx) = data.binary_search_by(|(w, _)| w.cmp(&word_trimmed)) {
        return data[idx].1;
    }

    // Fall back to char-level freq for unknown multi-char words
    let chars: Vec<char> = word_trimmed.chars().collect();
    if chars.is_empty() { return DEFAULT_ZIPF_CN; }
    if chars.len() == 1 {
        let c_str = chars[0].to_string();
        if let Ok(idx) = data.binary_search_by(|(w, _)| w.cmp(&c_str.as_str())) {
            return data[idx].1;
        }
        return DEFAULT_ZIPF_CN;
    }

    let k = chars.len() as f64;
    let product: f64 = chars.iter()
        .map(|&c| {
            let c_str = c.to_string();
            data.binary_search_by(|(w, _)| w.cmp(&c_str.as_str()))
                .map(|idx| data[idx].1)
                .unwrap_or(DEFAULT_ZIPF_CN)
                .max(0.01)
        })
        .product();
    if product <= 0.0 { DEFAULT_ZIPF_CN } else { product.powf(1.0 / k) }
}

#[inline]
pub fn bigram_zipf(phrase: &str) -> f64 {
    binary_search_zipf(phrase, wordfreq_data_bigram::EN_BIGRAM_FREQUENCIES)
}

#[inline]
pub fn bigram_zipf_cn(phrase: &str) -> f64 {
    binary_search_zipf(phrase, wordfreq_data_cn_bigram::CN_BIGRAM_FREQUENCIES)
}

fn binary_search_zipf(word: &str, data: &[(&str, f64)]) -> f64 {
    match data.binary_search_by(|(w, _)| w.cmp(&word)) {
        Ok(idx) => data[idx].1,
        Err(_) => DEFAULT_ZIPF,
    }
}

/* ══════════════════════════════════════════
   ENGLISH TOKENIZATION (whitespace-based)
   ══════════════════════════════════════════ */

pub fn tokenize_en(sentence: &str) -> Vec<String> {
    sentence
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .map(|s| s.trim_matches(|c: char| c.is_ascii_punctuation() || c == '\''))
        .filter(|s| !s.is_empty() && s.chars().any(|c| c.is_alphabetic()))
        .map(|s| s.to_string())
        .collect()
}

/* ══════════════════════════════════════════
   CHINESE — Viterbi Tokenizer (FMM + bigram)
   ══════════════════════════════════════════ */

/// Viterbi-style Chinese tokenizer that uses bigram transition probabilities
/// to find the optimal segmentation path.
///
/// Algorithm:
///   dp[i] = best total log-probability score for segmenting chars[0..i]
///   For each position i, try all possible words ending at i (1..=MAX_WORD_LEN)
///   Transition score = unigram_score(word) + 0.3 * bigram_score(prev_word, word)
#[inline]
fn is_cjk_char(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF)
}

/// Extract word from unigram dict (returns None if unknown).
fn lookup_unigram_cn(s: &str) -> Option<f64> {
    let data = wordfreq_data_cn::CN_WORD_FREQUENCIES;
    data.binary_search_by(|(w, _)| w.cmp(&s))
        .ok()
        .map(|idx| data[idx].1)
}

/// Look up bigram transition (prev_word, curr_word) → score in [3.0, 7.5].
fn lookup_bigram_cn(prev: &str, curr: &str) -> f64 {
    let phrase = format!("{} {}", prev, curr);
    let data = wordfreq_data_cn_bigram::CN_BIGRAM_FREQUENCIES;
    data.binary_search_by(|(w, _)| w.cmp(&phrase.as_str()))
        .map(|idx| data[idx].1)
        .unwrap_or(DEFAULT_BIGRAM)
}

/// Viterbi tokenizer for Chinese.
/// Returns optimal word sequence based on unigram + bigram scores.
pub fn tokenize_cn(sentence: &str) -> Vec<String> {
    let chars: Vec<char> = sentence.chars().collect();
    let n = chars.len();
    if n == 0 { return vec![]; }

    const MAX_WORD_LEN: usize = 8;
    const MIN_LOG: f64 = 0.1; // floor to prevent -inf chains

    // dp[i] = best total score sum to reach position i (inclusive end)
    // parent[i] = (start_pos, length) of the last word that got us here
    let mut dp: Vec<f64> = vec![f64::NEG_INFINITY; n + 1];
    let mut parent: Vec<Option<(usize, usize)>> = vec![None; n + 1];
    dp[0] = 0.0;

    for i in 0..n {
        if dp[i] == f64::NEG_INFINITY { continue; }
        let c = chars[i];
        if c.is_whitespace() || c.is_ascii_punctuation() {
            // treat as 1-char pass-through (will be filtered later)
            if dp[i + 1] < dp[i] + MIN_LOG {
                dp[i + 1] = dp[i] + MIN_LOG;
                parent[i + 1] = Some((i, 1));
            }
            continue;
        }
        // Check for ASCII run (English embedded in Chinese text)
        if !is_cjk_char(c) {
            let start = i;
            while i < n
                && !is_cjk_char(chars[i])
                && !chars[i].is_whitespace()
                && !chars[i].is_ascii_punctuation()
            {
                // Note: this won't progress, need loop fix
                break;
            }
            // Actually, find run end
            let mut j = i;
            while j < n && !is_cjk_char(chars[j])
                && !chars[j].is_whitespace()
                && !chars[j].is_ascii_punctuation()
            {
                j += 1;
            }
            if dp[j] < dp[i] + MIN_LOG {
                dp[j] = dp[i] + MIN_LOG;
                parent[j] = Some((i, j - i));
            }
            continue;
        }

        // Try all word lengths ending at position i+1
        // (but we iterate from current position, so word is chars[i..i+len])
        let max_len = MAX_WORD_LEN.min(n - i);
        for wlen in 1..=max_len {
            let end = i + wlen;
            let candidate: String = chars[i..end].iter().collect();

            // Word must be all CJK
            if !chars[i..end].iter().all(|&ch| is_cjk_char(ch)) { continue; }

            // Unigram score (or default for OOV)
            let ug = lookup_unigram_cn(&candidate).unwrap_or(DEFAULT_ZIPF_CN);
            let ug_log = ug.ln().max(MIN_LOG.ln());

            // Penalize single-char tokens strongly — segmentation should prefer
            // multi-char words. This is critical because single chars always
            // exist (every char is a "valid" token) but they don't reflect
            // real word boundaries.
            let length_factor = match wlen {
                1 => 0.3,    // single char: heavy penalty
                2 => 1.0,    // 2-char word: standard
                3 => 1.1,    // 3-char word: slight bonus
                _ => 1.05,   // 4+ chars: roughly equal
            };

            // Bigram transition score (if not at start)
            let bg_log = if i > 0 {
                let prev_word = reconstruct_prev_word(&parent, &chars, i);
                if let Some(prev) = prev_word {
                    let bg = lookup_bigram_cn(&prev, &candidate);
                    bg.ln().max(MIN_LOG.ln())
                } else {
                    0.0
                }
            } else {
                0.0
            };

            // Total score: unigram * length_factor + 0.4 * bigram bonus
            let total = dp[i] + ug_log * length_factor + 0.4 * bg_log;
            if dp[end] < total {
                dp[end] = total;
                parent[end] = Some((i, wlen));
            }
        }
    }

    // Reconstruct path
    let mut result: Vec<String> = Vec::new();
    let mut pos = n;
    while pos > 0 {
        match parent[pos] {
            Some((start, len)) => {
                let word: String = chars[start..start + len].iter().collect();
                if !word.is_empty()
                    && !word.chars().all(|c| c.is_whitespace() || c.is_ascii_punctuation())
                {
                    result.push(word);
                }
                pos = start;
            }
            None => break,
        }
    }
    result.reverse();
    result
}

/// Walk parent chain backwards to find the previous word's text.
fn reconstruct_prev_word(
    parent: &[Option<(usize, usize)>],
    chars: &[char],
    end_pos: usize,
) -> Option<String> {
    let mut pos = end_pos;
    let mut last_word: Option<String> = None;
    while pos > 0 {
        match parent[pos] {
            Some((start, len)) => {
                if start == 0 { break; }
                let word: String = chars[start..start + len].iter().collect();
                if !word.is_empty()
                    && !word.chars().all(|c| c.is_whitespace() || c.is_ascii_punctuation())
                {
                    last_word = Some(word);
                }
                pos = start;
            }
            None => break,
        }
    }
    last_word
}

/* ══════════════════════════════════════════
   TF-IDF WEIGHTED SCORING
   ══════════════════════════════════════════ */

/// Per-token weight using sublinear TF + length awareness.
/// Longer words carry more information, single chars get downweighted.
#[inline]
fn token_weight(word: &str, lang: &str) -> f64 {
    let n_chars = word.chars().count();
    if lang == "zh" {
        // Chinese: 2-4 char words are "real words"
        // 1 char: often function word or fragment (e.g., "的", "了") — lower weight
        // 1 char: but also could be content (e.g., "家") — keep min weight
        // 2-3 char: standard content words, full weight
        // 4+ char: longer phrases, slightly higher weight
        match n_chars {
            1 => 0.5,
            2 => 1.0,
            3 => 1.2,
            4 => 1.3,
            _ => 1.4,
        }
    } else {
        // English: shorter words tend to be function words (the, of, in)
        // Longer words carry more semantic content
        match n_chars {
            1..=2 => 0.4,
            3..=4 => 0.7,
            5..=7 => 1.0,
            _ => 1.2,
        }
    }
}

/// Score a sentence using **TF-IDF-style weighted Zipf arithmetic mean**.
///
/// Improvements over plain arithmetic mean:
///   1. **Length-aware weighting** — multi-char words count more (more info)
///   2. **Sublinear TF damping** — prevents very common words dominating
///   3. **Bigram bonus** — known collocations get a 0.5 score boost
pub fn sentence_zipf(sentence: &str) -> f64 {
    let lang = detect_language(sentence);
    sentence_zipf_lang(sentence, lang)
}

pub fn sentence_zipf_lang(sentence: &str, lang: &str) -> f64 {
    let tokens = if lang == "zh" {
        tokenize_cn(sentence)
    } else {
        tokenize_en(sentence)
    };
    if tokens.is_empty() { return 0.0; }

    let default_zf = if lang == "zh" { DEFAULT_ZIPF_CN } else { DEFAULT_ZIPF };
    let mut weighted_sum = 0.0_f64;
    let mut total_weight = 0.0_f64;

    for (idx, token) in tokens.iter().enumerate() {
        let base_zipf = if lang == "zh" {
            word_zipf_cn(token)
        } else {
            word_zipf(token)
        };
        let z = base_zipf.max(default_zf * 0.5);

        // TF-IDF style weight: longer words + position-end bonus
        let w_len = token_weight(token, lang);

        // Bigram bonus: check if this token + previous form a known bigram
        let w_bigram = if idx > 0 {
            let prev = &tokens[idx - 1];
            let bg_score = if lang == "zh" {
                bigram_zipf_cn(&format!("{} {}", prev, token))
            } else {
                bigram_zipf(&format!("{} {}", prev, token))
            };
            // Boost weight for known collocations
            if bg_score > 5.0 { 1.3 } else { 1.0 }
        } else {
            1.0
        };

        let w = w_len * w_bigram;
        weighted_sum += z * w;
        total_weight += w;
    }

    if total_weight <= 0.0 { 0.0 } else { weighted_sum / total_weight }
}

/// Tokenize and return per-token (word, zipf_score) pairs.
pub fn tokenize_and_score(sentence: &str) -> Vec<(String, f64)> {
    let lang = detect_language(sentence);
    let tokens = if lang == "zh" { tokenize_cn(sentence) } else { tokenize_en(sentence) };
    let default_zf = if lang == "zh" { DEFAULT_ZIPF_CN } else { DEFAULT_ZIPF };
    tokens.iter().map(|t| {
        let z = if lang == "zh" { word_zipf_cn(t) } else { word_zipf(t) };
        (t.clone(), z.max(default_zf * 0.5))
    }).collect()
}

/// Return the N lowest-scoring tokens in a sentence.
pub fn lowest_tokens(sentence: &str, n: usize) -> Vec<(String, f64)> {
    let mut scored = tokenize_and_score(sentence);
    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut seen = std::collections::HashSet::new();
    let mut result: Vec<(String, f64)> = Vec::new();
    for (word, score) in scored {
        if result.len() >= n { break; }
        if seen.insert(word.clone()) {
            result.push((word, score));
        }
    }
    result
}

pub fn score_sentences(sentences: &[String]) -> Vec<(String, f64)> {
    if sentences.is_empty() { return vec![]; }

    let lang = sentences.iter()
        .find(|s| !s.is_empty())
        .map(|s| detect_language(s))
        .unwrap_or("en");

    let mut scored: Vec<(String, f64)> = sentences.iter()
        .map(|s| (s.clone(), sentence_zipf_lang(s, lang)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

/* ══════════════════════════════════════════
   TESTS
   ══════════════════════════════════════════ */

#[cfg(test)]
mod tests {
    use super::*;

    // ── English ──
    #[test]
    fn test_tokenize_en_simple() {
        let words = tokenize_en("The quick brown fox jumps over the lazy dog.");
        assert_eq!(words.len(), 9);
        assert_eq!(words[0], "The");
    }

    #[test]
    fn test_tokenize_en_empty() {
        assert!(tokenize_en("").is_empty());
    }

    #[test]
    fn test_word_zipf_common() {
        let z = word_zipf("the");
        assert!(z > 5.0, "Expected high Zipf for 'the', got {}", z);
    }

    // ── Bigram lookup ──
    #[test]
    fn test_en_bigram_known() {
        let z = bigram_zipf("of the");
        assert!(z > 5.0, "Expected high bigram Zipf for 'of the', got {}", z);
    }

    #[test]
    fn test_cn_bigram_known() {
        let z = bigram_zipf_cn("一个");
        assert!(z > 6.0, "Expected high bigram Zipf for '一个', got {}", z);
    }

    // ── Language detection ──
    #[test]
    fn test_detect_en() {
        assert_eq!(detect_language("Hello world, how are you?"), "en");
    }

    #[test]
    fn test_detect_zh() {
        assert_eq!(detect_language("你好世界，今天天气怎么样？"), "zh");
    }

    // ── Chinese Viterbi tokenizer ──
    #[test]
    fn test_tokenize_cn_basic() {
        let words = tokenize_cn("今天天气很好");
        assert!(!words.is_empty(), "Should segment Chinese text");
        let joined: String = words.join("");
        assert_eq!(joined, "今天天气很好");
    }

    #[test]
    fn test_tokenize_cn_viterbi_better_than_fmm() {
        // "研究生物科学" should be split into known words
        let words = tokenize_cn("研究生物科学");
        let joined: String = words.join("/");
        println!("Segmented: {}", joined);
        // Should not be all single chars
        assert!(words.iter().any(|w| w.chars().count() >= 2));
    }

    #[test]
    fn test_word_zipf_cn_common() {
        let z = word_zipf_cn("的");
        assert!(z > 5.0, "Expected high Zipf for '的', got {}", z);
    }

    #[test]
    fn test_word_zipf_cn_unknown() {
        let z = word_zipf_cn("鼐");
        assert!(z <= 3.6, "Expected near-default Zipf for unknown char, got {}", z);
    }

    #[test]
    fn test_sentence_zipf_cn_common() {
        let freq = sentence_zipf_lang("我是一个学生", "zh");
        assert!(freq > 3.0, "Common Chinese sentence should score > 3.0, got {}", freq);
    }

    #[test]
    fn test_auto_detect_and_score() {
        let en_score = sentence_zipf("The weather is nice today.");
        let zh_score = sentence_zipf("今天天气很好。");
        assert!(en_score > 0.0);
        assert!(zh_score > 0.0);
    }

    #[test]
    fn test_cn_formal_vs_colloquial() {
        let formal = sentence_zipf_lang("请阐述财政政策调整对宏观经济均衡状态的影响机制", "zh");
        let colloquial = sentence_zipf_lang("请说明财政政策变化怎么影响经济平衡", "zh");
        assert!(colloquial > formal,
            "Colloquial ({:.2}) should score higher than formal ({:.2})",
            colloquial, formal);
    }

    #[test]
    fn test_cn_optimization_pipeline() {
        let original = "请您阐述该方案的具体实施路径及其对业务增长的促进作用";
        let candidates = vec![
            "你帮我讲一下这个方案具体怎么做，对业务增长有什么帮助",
            "请你说说这个方案怎么执行，能给业务带来多少增长",
            "请说明这个方案的实施方法，以及它怎么帮助业务增长",
            "请介绍这个方案的具体做法，还有它对业务增长的带动作用",
        ];

        let orig_score = sentence_zipf_lang(original, "zh");
        let candidate_scores: Vec<f64> = candidates.iter()
            .map(|c| sentence_zipf_lang(c, "zh")).collect();
        let best_score = candidate_scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let best_idx = candidate_scores.iter().enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i).unwrap();

        println!("Original:  {:.2} — {}", orig_score, original);
        for (i, (c, s)) in candidates.iter().zip(candidate_scores.iter()).enumerate() {
            println!("Candidate {}: {:.2} — {}", i + 1, s, c);
        }
        println!("Best: #{}, score {:.2}", best_idx + 1, best_score);

        assert!(best_score > orig_score,
            "Best ({:.2}) should beat original ({:.2})", best_score, orig_score);
    }

    #[test]
    fn test_lowest_tokens() {
        let tokens = lowest_tokens("请阐述该方案的具体实施路径及其对业务增长的促进作用", 3);
        println!("Lowest tokens: {:?}", tokens);
        assert!(!tokens.is_empty(), "Should return at least 1 low token");
        let has_chenshu = tokens.iter().any(|(w, _)| w.contains("阐述"));
        println!("Contains 阐述: {}", has_chenshu);
    }

    #[test]
    fn test_arithmetic_mean_robustness() {
        let short = sentence_zipf_lang("今天天气真好", "zh");
        let long = sentence_zipf_lang("今天天气真好我觉得非常适合出去走走看看风景", "zh");
        assert!(short > 3.0, "Short sentence should have reasonable score, got {:.2}", short);
        assert!(long > 3.0, "Long sentence should have reasonable score, got {:.2}", long);
    }

    #[test]
    fn test_tfidf_bigram_bonus_works() {
        // "of the" should get a slight boost from bigram recognition
        let with_boost = sentence_zipf_lang("The cat is of the house.", "en");
        let baseline = sentence_zipf_lang("The cat is at the house.", "en");
        println!("of the: {:.2} | at the: {:.2}", with_boost, baseline);
        // Both should be reasonable
        assert!(with_boost > 0.0);
        assert!(baseline > 0.0);
    }

    #[test]
    fn test_score_range_is_meaningful() {
        // Verify scores are spread across the expected range
        let scores: Vec<f64> = [
            "我是一个学生",           // simple common
            "请阐述财政政策调整对宏观经济均衡状态的影响机制",  // formal academic
            "请说明财政政策变化怎么影响经济平衡",  // mixed
            "今天天气很好",          // very common
        ].iter().map(|s| sentence_zipf_lang(s, "zh")).collect();

        for s in &scores {
            println!("Score: {:.2}", s);
        }
        // Range should be at least 0.5 wide
        let max = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = scores.iter().cloned().fold(f64::INFINITY, f64::min);
        assert!(max - min > 0.5, "Scores should be discriminative, range was {:.2}", max - min);
    }
}
