//! English slot extractor.
//!
//! Rule-based POS + heuristic slot assignment. Lightweight, no external
//! dependencies; ships in WASM.
//!
//! ## Strategy
//!
//! 1. Tokenize on whitespace + ASCII punctuation
//! 2. Tag each token with POS (suffix-based + known-words list)
//! 3. Map POS + position to SlotKind, mirroring the Chinese extractor
//! 4. Detect special markers: please → Register, not/never → Negation,
//!    if/when → Condition, how/why/what → Qtype
//! 5. Capitalized non-initial words → Entity

use crate::pos::{tag_en, Pos};
use crate::slot::{Slot, SlotKind};

/// Extract slots from an English prompt.
pub fn extract_slots_en(text: &str) -> Vec<Slot> {
    let raw_tokens = tokenize_en_with_offsets(text);
    if raw_tokens.is_empty() { return vec![]; }

    let mut slots: Vec<Slot> = Vec::new();
    let n = raw_tokens.len();
    let pos_tags: Vec<Pos> = raw_tokens.iter().map(|t| tag_en(&t.text)).collect();

    for (idx, (tok, &pos)) in raw_tokens.iter().zip(pos_tags.iter()).enumerate() {
        let lower = tok.text.to_lowercase();

        // Politeness / register
        if matches!(lower.as_str(), "please" | "pls" | "kindly" | "would" | "could") {
            slots.push(Slot::new(SlotKind::Register, tok.text.clone(), tok.start, tok.end)
                .with_sub_tag("politeness"));
            continue;
        }

        // Negation
        if matches!(lower.as_str(), "not" | "no" | "never" | "neither" | "nor" | "n't" | "cannot")
            || (lower.ends_with("n't") && lower.len() > 4)
        {
            slots.push(Slot::new(SlotKind::Negation, tok.text.clone(), tok.start, tok.end));
            continue;
        }

        // Conditional
        if matches!(lower.as_str(), "if" | "when" | "whenever" | "unless" | "assuming" | "suppose")
        {
            slots.push(Slot::new(SlotKind::Condition, tok.text.clone(), tok.start, tok.end));
            continue;
        }

        // Q-type markers
        if is_qtype_marker_en(&lower) {
            slots.push(Slot::new(SlotKind::Qtype, tok.text.clone(), tok.start, tok.end));
            continue;
        }

        // Capitalized mid-sentence → named entity
        let is_sentence_start = idx == 0
            || (idx > 0 && raw_tokens[idx - 1].text.ends_with('.'))
            || (idx > 0 && raw_tokens[idx - 1].text.ends_with('!'))
            || (idx > 0 && raw_tokens[idx - 1].text.ends_with('?'));
        if !is_sentence_start
            && tok.text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && tok.text.chars().all(|c| c.is_alphabetic())
        {
            slots.push(Slot::new(SlotKind::Entity, tok.text.clone(), tok.start, tok.end)
                .with_sub_tag("proper-noun"));
            continue;
        }

        // Action: verb in the first half of the sentence, OR a verb directly
        // followed by a noun (likely "X is/are/was Y")
        if pos == Pos::Verb && (idx <= n / 2) {
            slots.push(Slot::new(SlotKind::Action, tok.text.clone(), tok.start, tok.end)
                .with_sub_tag("verb"));
            continue;
        }

        // Action: verb in the first half of the sentence, OR a verb directly
        // followed by a noun (likely "X is/are/was Y")
        if pos == Pos::Verb && (idx <= n / 2) {
            slots.push(Slot::new(SlotKind::Action, tok.text.clone(), tok.start, tok.end)
                .with_sub_tag("verb"));
            continue;
        }

        // Modifier (adjective) — emit so it can merge with following Object
        if pos == Pos::Adjective {
            slots.push(Slot::new(SlotKind::Modifier, tok.text.clone(), tok.start, tok.end)
                .with_sub_tag("adj"));
            continue;
        }

        // Object: abstract noun not at the start
        if pos == Pos::Noun && !is_sentence_start && !is_function_like_noun_en(&lower) {
            slots.push(Slot::new(SlotKind::Object, tok.text.clone(), tok.start, tok.end)
                .with_sub_tag("noun"));
            continue;
        }
    }

    // Post-process: merge adjacent Object slots
    merge_adjacent_objects_en(&mut slots);

    slots
}

#[derive(Debug, Clone)]
struct RawToken {
    text: String,
    start: usize, // char offset
    end: usize,
}

fn tokenize_en_with_offsets(text: &str) -> Vec<RawToken> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut current_start: Option<usize> = None;
    let mut char_idx = 0;

    for c in text.chars() {
        let is_sep = c.is_whitespace() || c.is_ascii_punctuation();
        if is_sep {
            if !current.is_empty() {
                out.push(RawToken {
                    text: current.clone(),
                    start: current_start.unwrap(),
                    end: char_idx,
                });
                current.clear();
                current_start = None;
            }
        } else {
            if current.is_empty() {
                current_start = Some(char_idx);
            }
            current.push(c);
        }
        char_idx += 1;
    }
    if !current.is_empty() {
        out.push(RawToken {
            text: current,
            start: current_start.unwrap(),
            end: char_idx,
        });
    }
    out
}

fn is_qtype_marker_en(token: &str) -> bool {
    // Question words and answer-shape nouns (not verbs — those are Action)
    matches!(token,
        "how" | "why" | "what" | "when" | "where" | "who" | "which" |
        "mechanism" | "process" | "method" | "procedure" | "reason" | "cause"
    )
}

fn is_function_like_noun_en(token: &str) -> bool {
    // Words that look like nouns but are too generic to be slot-critical
    matches!(token, "thing" | "way" | "kind" | "sort" | "type" | "one" | "ones" | "stuff")
}

fn merge_adjacent_objects_en(slots: &mut Vec<Slot>) {
    let mut i = 0;
    while i < slots.len() {
        // Merge a Modifier (adjective) followed by an Object into a single Object phrase
        // (e.g., "fiscal policy" → one Object slot)
        if slots[i].kind == SlotKind::Modifier
            && i + 1 < slots.len()
            && slots[i + 1].kind == SlotKind::Object
            && slots[i + 1].char_start == slots[i].char_end + 1
        {
            let start = slots[i].char_start;
            let end = slots[i + 1].char_end;
            let merged = format!("{} {}", slots[i].text, slots[i + 1].text);
            slots[i] = Slot::new(SlotKind::Object, merged, start, end)
                .with_sub_tag("phrase");
            slots.remove(i + 1);
            continue;
        }

        if slots[i].kind == SlotKind::Object {
            let mut j = i + 1;
            while j < slots.len()
                && slots[j].kind == SlotKind::Object
                && slots[j].char_start == slots[j - 1].char_end + 1
            {
                j += 1;
            }
            if j > i + 1 {
                let start = slots[i].char_start;
                let end = slots[j - 1].char_end;
                let merged: String = slots[i..j].iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<&str>>()
                    .join(" ");
                slots[i] = Slot::new(SlotKind::Object, merged, start, end)
                    .with_sub_tag("phrase");
                slots.drain(i + 1..j);
            }
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explain_prompt() {
        let text = "Please explain the impact of fiscal policy on macroeconomic equilibrium.";
        let slots = extract_slots_en(text);
        println!("Slots: {:#?}", slots);

        let kinds: Vec<&'static str> = slots.iter().map(|s| s.kind.tag()).collect();
        assert!(kinds.contains(&"register"), "Should detect Please as Register");
        assert!(kinds.contains(&"action"), "Should detect explain as Action");
        assert!(kinds.contains(&"object"), "Should detect nouns as Object");
    }

    #[test]
    fn test_narrowing_caught() {
        let original = "Please explain the impact of fiscal policy on macroeconomic equilibrium.";
        let bad = "Please explain the impact of national tax rules on overall economic balance.";

        let orig = extract_slots_en(original);
        let bad_s = extract_slots_en(bad);

        let orig_obj: Vec<&str> = orig.iter()
            .filter(|s| s.kind == SlotKind::Object)
            .map(|s| s.text.as_str())
            .collect();
        let bad_obj: Vec<&str> = bad_s.iter()
            .filter(|s| s.kind == SlotKind::Object)
            .map(|s| s.text.as_str())
            .collect();

        println!("Original: {:?}", orig_obj);
        println!("Bad:      {:?}", bad_obj);
        assert!(orig_obj.iter().any(|o| o.to_lowercase().contains("fiscal policy")));
        assert!(bad_obj.iter().any(|o| o.to_lowercase().contains("tax rules")));
    }

    #[test]
    fn test_negation_caught() {
        let s = extract_slots_en("Do not use this method");
        let has_neg = s.iter().any(|x| x.kind == SlotKind::Negation);
        assert!(has_neg, "Should detect 'not' as Negation");
    }

    #[test]
    fn test_qtype_caught() {
        let s = extract_slots_en("How does this work?");
        let has_qt = s.iter().any(|x| x.kind == SlotKind::Qtype);
        assert!(has_qt, "Should detect 'How' as Qtype");
    }

    #[test]
    fn test_proper_noun_caught() {
        let s = extract_slots_en("Tell me about Apple Inc and Microsoft");
        let ents: Vec<&str> = s.iter()
            .filter(|x| x.kind == SlotKind::Entity)
            .map(|x| x.text.as_str())
            .collect();
        assert!(ents.contains(&"Apple"), "Should detect Apple as Entity");
        assert!(ents.contains(&"Microsoft"), "Should detect Microsoft as Entity");
    }

    #[test]
    fn test_conditional_caught() {
        let s = extract_slots_en("If you have time, tell me");
        let has_cond = s.iter().any(|x| x.kind == SlotKind::Condition);
        assert!(has_cond, "Should detect 'If' as Condition");
    }

    #[test]
    fn test_register_marker() {
        let s = extract_slots_en("Please do this carefully");
        let has_reg = s.iter().any(|x| x.kind == SlotKind::Register);
        assert!(has_reg, "Should detect Please as Register");
    }

    #[test]
    fn test_empty() {
        let s = extract_slots_en("");
        assert!(s.is_empty());
    }

    #[test]
    fn test_simple_subject() {
        let s = extract_slots_en("I am a student");
        let obj = s.iter().find(|x| x.kind == SlotKind::Object);
        assert!(obj.is_some(), "Should detect 'student' as Object");
    }
}
