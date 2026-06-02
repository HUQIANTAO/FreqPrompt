mod wordfreq_data;
mod wordfreq_data_cn;

/// Default Zipf frequency for unknown English words.
pub const DEFAULT_ZIPF: f64 = 1.0;

/// Default Zipf for unknown Chinese words.
/// Chinese Zipf distribution has a steeper slope (s ≈ 1.3 vs English s ≈ 1.0),
/// so rare words are even rarer. Use a slightly lower default.
pub const DEFAULT_ZIPF_CN: f64 = 0.8;

/* ══════════════════════════════════════════
   LANGUAGE DETECTION
   ══════════════════════════════════════════ */

/// Detect the primary language of a text.
/// Returns "zh" if >30% of characters are in the CJK Unified Ideographs range,
/// "en" otherwise.
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
                '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
                    | '\u{3400}'..='\u{4DBF}'   // CJK Extension A
                    | '\u{F900}'..='\u{FAFF}'   // CJK Compatibility
                    | '\u{3000}'..='\u{303F}'   // CJK Punctuation
                    | '\u{FF00}'..='\u{FFEF}'   // Fullwidth forms
            )
        })
        .count();
    let ratio = cjk_count as f64 / total_chars as f64;
    if ratio > 0.3 { "zh" } else { "en" }
}

/* ══════════════════════════════════════════
   ENGLISH
   ══════════════════════════════════════════ */

pub fn word_zipf(word: &str) -> f64 {
    let word_lower = word.to_lowercase();
    binary_search_zipf(&word_lower, wordfreq_data::WORD_FREQUENCIES)
}

fn binary_search_zipf(word: &str, data: &[(&str, f64)]) -> f64 {
    match data.binary_search_by(|(w, _)| w.cmp(&word)) {
        Ok(idx) => data[idx].1,
        Err(_) => DEFAULT_ZIPF,
    }
}

pub fn tokenize_en(sentence: &str) -> Vec<String> {
    sentence
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .map(|s| s.trim_matches(|c: char| c.is_ascii_punctuation() || c == '\''))
        .filter(|s| !s.is_empty() && s.chars().any(|c| c.is_alphabetic()))
        .map(|s| s.to_string())
        .collect()
}

/* ══════════════════════════════════════════
   CHINESE — Pure Rust FMM Tokenizer
   ══════════════════════════════════════════ */

/// Build a set of known Chinese words for maximum matching.
/// We use the frequency dictionary as our word list.
fn cn_word_set() -> Vec<&'static str> {
    wordfreq_data_cn::CN_WORD_FREQUENCIES
        .iter()
        .map(|(w, _)| *w)
        .collect()
}

/// Forward Maximum Matching (FMM) Chinese tokenizer.
///
/// Algorithm:
/// 1. Start from the beginning of the text
/// 2. Look for the longest word in the dictionary that matches at the current position
/// 3. If found, emit it as a token; otherwise emit the single character
/// 4. Advance and repeat
///
/// This is a standard, well-established Chinese segmentation algorithm.
/// For our frequency-scoring use case, it provides accurate enough segmentation
/// without external dependencies (unlike jieba which requires C compilation).
pub fn tokenize_cn(sentence: &str) -> Vec<String> {
    let dict = cn_word_set();
    // Sort by length descending so we can try longest match first
    // (the dictionary is already pre-sorted by Unicode code point, not length,
    // but we need to find max-length matches efficiently)
    //
    // Strategy: for each position, scan the dictionary for words that match
    // and pick the longest. For efficiency, we build a length-sorted reference.

    let chars: Vec<char> = sentence.chars().collect();
    let len = chars.len();
    let mut tokens: Vec<String> = Vec::new();
    let mut i = 0;

    // Max word length in our dictionary (Chinese words are typically 1-4 chars)
    const MAX_WORD_LEN: usize = 6;

    while i < len {
        // Skip whitespace and punctuation
        if chars[i].is_whitespace() || chars[i].is_ascii_punctuation() {
            i += 1;
            continue;
        }

        // Skip CJK punctuation
        if matches!(chars[i] as u32, 0x3000..=0x303F | 0xFF00..=0xFF0F | 0xFF1A..=0xFF20 | 0xFF3B..=0xFF40 | 0xFF5B..=0xFF65) {
            i += 1;
            continue;
        }

        let mut best_match_len: usize = 1;
        let remaining = len - i;

        // Try progressively shorter substrings, starting from longest
        for wlen in (2..=MAX_WORD_LEN.min(remaining)).rev() {
            let candidate: String = chars[i..i + wlen].iter().collect();
            // Binary search in our sorted dictionary
            if dict.binary_search(&candidate.as_str()).is_ok() {
                best_match_len = wlen;
                break;
            }
        }

        let token: String = chars[i..i + best_match_len].iter().collect();
        tokens.push(token);
        i += best_match_len;
    }

    tokens
}

/// Look up the Zipf frequency of a Chinese word.
/// For multi-character words not in the dictionary, uses character-level fallback.
pub fn word_zipf_cn(word: &str) -> f64 {
    let word_trimmed = word.trim();
    if word_trimmed.is_empty() {
        return DEFAULT_ZIPF_CN;
    }

    let data = wordfreq_data_cn::CN_WORD_FREQUENCIES;

    // Try exact match
    if let Ok(idx) = data.binary_search_by(|(w, _)| w.cmp(&word_trimmed)) {
        return data[idx].1;
    }

    // Fallback: geometric mean of individual character frequencies
    let chars: Vec<char> = word_trimmed.chars().collect();
    if chars.is_empty() {
        return DEFAULT_ZIPF_CN;
    }
    if chars.len() == 1 {
        // Single char — look it up directly
        let c_str = chars[0].to_string();
        if let Ok(idx) = data.binary_search_by(|(w, _)| w.cmp(&c_str.as_str())) {
            return data[idx].1;
        }
        return DEFAULT_ZIPF_CN;
    }

    // Multi-char unknown word: geometric mean of char scores
    let k = chars.len() as f64;
    let product: f64 = chars
        .iter()
        .map(|&c| {
            let c_str = c.to_string();
            data.binary_search_by(|(w, _)| w.cmp(&c_str.as_str()))
                .map(|idx| data[idx].1)
                .unwrap_or(DEFAULT_ZIPF_CN)
                .max(0.01)
        })
        .product();

    if product <= 0.0 {
        return DEFAULT_ZIPF_CN;
    }
    product.powf(1.0 / k)
}

/* ══════════════════════════════════════════
   LANGUAGE-AWARE SCORING
   ══════════════════════════════════════════ */

pub fn sentence_zipf(sentence: &str) -> f64 {
    let lang = detect_language(sentence);
    sentence_zipf_lang(sentence, lang)
}

pub fn sentence_zipf_lang(sentence: &str, lang: &str) -> f64 {
    let (words, default_zf) = if lang == "zh" {
        (tokenize_cn(sentence), DEFAULT_ZIPF_CN)
    } else {
        (tokenize_en(sentence), DEFAULT_ZIPF)
    };

    if words.is_empty() {
        return 0.0;
    }

    let k = words.len() as f64;
    let product: f64 = words
        .iter()
        .map(|w| {
            let z = if lang == "zh" {
                word_zipf_cn(w)
            } else {
                word_zipf(w)
            };
            z.max(default_zf)
        })
        .product();

    if product <= 0.0 {
        return 0.0;
    }
    product.powf(1.0 / k)
}

pub fn score_sentences(sentences: &[String]) -> Vec<(String, f64)> {
    if sentences.is_empty() {
        return vec![];
    }
    let lang = sentences
        .iter()
        .find(|s| !s.is_empty())
        .map(|s| detect_language(s))
        .unwrap_or("en");

    let mut scored: Vec<(String, f64)> = sentences
        .iter()
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

    // ── Language detection ──
    #[test]
    fn test_detect_en() {
        assert_eq!(detect_language("Hello world, how are you?"), "en");
    }

    #[test]
    fn test_detect_zh() {
        assert_eq!(detect_language("你好世界，今天天气怎么样？"), "zh");
    }

    // ── Chinese FMM tokenizer ──
    #[test]
    fn test_tokenize_cn_basic() {
        let words = tokenize_cn("今天天气很好");
        assert!(!words.is_empty(), "Should segment Chinese text");
    }

    #[test]
    fn test_tokenize_cn_known_words() {
        let words = tokenize_cn("我是一个学生");
        // Should contain known words like 我, 是, 一个, 学生
        assert!(words.len() >= 2);
    }

    #[test]
    fn test_word_zipf_cn_common() {
        let z = word_zipf_cn("的");
        assert!(z > 5.0, "Expected high Zipf for '的', got {}", z);
    }

    #[test]
    fn test_word_zipf_cn_unknown() {
        // A very rare character should get a low score
        let z = word_zipf_cn("𬜬");
        assert!(z < 3.0, "Expected low Zipf for rare character, got {}", z);
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
}
