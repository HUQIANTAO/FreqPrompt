//! Domain-specific frequency table: built from a user corpus.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A word's frequency data in a domain corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordFreq {
    /// The word/token
    pub word: String,
    /// Raw count in the corpus
    pub count: u64,
    /// Zipf-scale frequency (log10(freq_per_billion) scaled)
    pub zipf: f64,
}

/// A domain-specific frequency table built from a corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainFreqTable {
    /// Domain name (e.g., "finance_corpus")
    pub name: String,
    /// Language ("zh" or "en")
    pub lang: String,
    /// Total token count in the corpus
    pub total_tokens: u64,
    /// Unique word count
    pub unique_words: u64,
    /// Word frequencies indexed by word
    pub words: HashMap<String, WordFreq>,
}

impl DomainFreqTable {
    /// Create a new empty table.
    pub fn new(name: String, lang: String) -> Self {
        Self {
            name,
            lang,
            total_tokens: 0,
            unique_words: 0,
            words: HashMap::new(),
        }
    }

    /// Build a frequency table from tokenized text.
    ///
    /// `tokens` is a list of pre-tokenized words from the corpus.
    pub fn from_tokens(name: String, lang: String, tokens: &[String]) -> Self {
        let mut counts: HashMap<String, u64> = HashMap::new();
        for tok in tokens {
            *counts.entry(tok.clone()).or_insert(0) += 1;
        }

        let total = tokens.len() as u64;
        let mut words = HashMap::new();

        for (word, count) in &counts {
            // Convert to Zipf scale: log10(count / total * 1e9) clamped to [0, 10]
            let freq_per_billion = (*count as f64) / (total as f64) * 1e9;
            let zipf = if freq_per_billion > 0.0 {
                freq_per_billion.log10().clamp(0.0, 10.0)
            } else {
                0.0
            };
            words.insert(
                word.clone(),
                WordFreq {
                    word: word.clone(),
                    count: *count,
                    zipf,
                },
            );
        }

        Self {
            name,
            lang,
            total_tokens: total,
            unique_words: counts.len() as u64,
            words,
        }
    }

    /// Look up a word's Zipf frequency. Returns None if not in table.
    pub fn zipf(&self, word: &str) -> Option<f64> {
        self.words.get(word).map(|w| w.zipf)
    }

    /// Get top-N words by frequency.
    pub fn top_n(&self, n: usize) -> Vec<&WordFreq> {
        let mut sorted: Vec<&WordFreq> = self.words.values().collect();
        sorted.sort_by(|a, b| b.count.cmp(&a.count));
        sorted.into_iter().take(n).collect()
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_tokens() {
        let tokens: Vec<String> = vec![
            "财政", "政策", "财政", "税收", "政策", "宏观", "经济",
            "财政", "政策", "均衡", "影响", "机制",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let table = DomainFreqTable::from_tokens("test".into(), "zh".into(), &tokens);

        assert_eq!(table.total_tokens, 12);
        assert_eq!(table.unique_words, 8); // 财政,政策,税收,宏观,经济,均衡,影响,机制
        assert!(table.zipf("财政").is_some());
        assert!(table.zipf("财政").unwrap() > table.zipf("均衡").unwrap());
        assert!(table.zipf("不存在").is_none());
    }

    #[test]
    fn test_top_n() {
        let tokens: Vec<String> = vec!["a", "b", "a", "c", "a", "b"]
            .into_iter()
            .map(String::from)
            .collect();
        let table = DomainFreqTable::from_tokens("test".into(), "en".into(), &tokens);
        let top = table.top_n(2);
        assert_eq!(top[0].word, "a");
        assert_eq!(top[0].count, 3);
        assert_eq!(top[1].word, "b");
        assert_eq!(top[1].count, 2);
    }

    #[test]
    fn test_serialization() {
        let tokens: Vec<String> = vec!["hello", "world", "hello"]
            .into_iter()
            .map(String::from)
            .collect();
        let table = DomainFreqTable::from_tokens("test".into(), "en".into(), &tokens);
        let json = table.to_json();
        let restored = DomainFreqTable::from_json(&json).unwrap();
        assert_eq!(restored.name, "test");
        assert_eq!(restored.total_tokens, 3);
        assert_eq!(restored.zipf("hello"), table.zipf("hello"));
    }
}
