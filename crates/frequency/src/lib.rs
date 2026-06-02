mod wordfreq_data;

/// Default Zipf frequency for unknown words (low frequency).
pub const DEFAULT_ZIPF: f64 = 1.0;

/// Compute the Zipf frequency of a single word.
/// Returns DEFAULT_ZIPF if the word is not in the dictionary.
pub fn word_zipf(word: &str) -> f64 {
    let word_lower = word.to_lowercase();
    binary_search_zipf(&word_lower)
}

/// Binary search in the sorted word-frequency array.
fn binary_search_zipf(word: &str) -> f64 {
    let data = wordfreq_data::WORD_FREQUENCIES;
    match data.binary_search_by(|(w, _)| w.cmp(&word)) {
        Ok(idx) => data[idx].1,
        Err(_) => DEFAULT_ZIPF,
    }
}

/// Tokenize a sentence into words.
/// Splits on whitespace and common punctuation, keeping only alphanumeric tokens.
pub fn tokenize(sentence: &str) -> Vec<String> {
    sentence
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .map(|s| s.trim_matches(|c: char| c.is_ascii_punctuation() || c == '\''))
        .filter(|s| !s.is_empty() && s.chars().any(|c| c.is_alphabetic()))
        .map(|s| s.to_string())
        .collect()
}

/// Compute the sentence-level frequency using the geometric mean of word frequencies.
///
/// Implements Equation 3 from the paper:
///   sfreq(x, D) = K√(∏ wfreq(x_k, D))
///
/// Where K is the number of words in the sentence.
pub fn sentence_zipf(sentence: &str) -> f64 {
    let words = tokenize(sentence);
    if words.is_empty() {
        return 0.0;
    }

    let k = words.len() as f64;
    let product: f64 = words.iter().map(|w| word_zipf(w)).product();

    // Geometric mean: product^(1/k)
    product.powf(1.0 / k)
}

/// Score a batch of sentences and return sorted (sentence, score) pairs, highest first.
pub fn score_sentences(sentences: &[String]) -> Vec<(String, f64)> {
    let mut scored: Vec<(String, f64)> = sentences
        .iter()
        .map(|s| (s.clone(), sentence_zipf(s)))
        .collect();
    // Sort by score descending (highest frequency first)
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let words = tokenize("The quick brown fox jumps over the lazy dog.");
        assert_eq!(words.len(), 9);
        assert_eq!(words[0], "The");
        assert_eq!(words[8], "dog");
    }

    #[test]
    fn test_tokenize_empty() {
        let words = tokenize("");
        assert!(words.is_empty());
    }

    #[test]
    fn test_word_zipf_common() {
        // "the" is the most frequent English word
        let z = word_zipf("the");
        assert!(z > 5.0, "Expected high Zipf for 'the', got {}", z);
    }

    #[test]
    fn test_word_zipf_unknown() {
        let z = word_zipf("xyzzynonexistentword");
        assert_eq!(z, DEFAULT_ZIPF);
    }

    #[test]
    fn test_sentence_zipf() {
        // A sentence with common words
        let freq_common = sentence_zipf("The cat sat on the mat.");
        // A sentence with rare words
        let freq_rare = sentence_zipf("Xylophone quixotic zephyrs ubiquitously jazz.");
        assert!(freq_common > freq_rare,
            "Expected common sentence ({}) > rare sentence ({})",
            freq_common, freq_rare);
    }

    #[test]
    fn test_score_sentences() {
        let sentences = vec![
            "ubiquitous perambulator".to_string(),
            "the cat sat".to_string(),
            "hello world".to_string(),
        ];
        let scored = score_sentences(&sentences);
        assert_eq!(scored.len(), 3);
        // First should have highest frequency
        assert!(scored[0].1 >= scored[1].1);
        assert!(scored[1].1 >= scored[2].1);
    }
}
