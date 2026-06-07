//! Chinese slot extractor.
//!
//! Uses the Viterbi tokenizer from the `frequency` crate, then assigns
//! each token to a `SlotKind` based on its POS tag, position, and
//! linguistic context.
//!
//! ## Position-finding strategy
//!
//! The Viterbi tokenizer returns a list of words. We walk the original
//! text char-by-char in lockstep with the token list, skipping whitespace
//! and punctuation, so each token gets its true position. This is more
//! robust than searching for each token from scratch (which fails when
//! the Viterbi split a word differently than expected).

use crate::pos::{tag_zh, Pos};
use crate::slot::{Slot, SlotKind};

/// Extract slots from a Chinese prompt.
pub fn extract_slots_zh(text: &str) -> Vec<Slot> {
    let raw_tokens = frequency::tokenize_cn(text);
    // Filter out pure-punctuation tokens (the upstream tokenizer may emit them
    // for CJK punctuation like "，" since its filter only checks ASCII)
    let tokens: Vec<String> = raw_tokens.into_iter()
        .filter(|t| !t.chars().all(|c| is_punctuation(c) || c.is_whitespace()))
        .collect();
    if tokens.is_empty() { return vec![]; }

    // Find positions sequentially
    let positions = compute_token_positions(text, &tokens);
    if positions.len() != tokens.len() {
        return vec![];
    }

    let mut slots: Vec<Slot> = Vec::new();
    let n = tokens.len();
    let pos_tags: Vec<Pos> = tokens.iter().map(|t| tag_zh(t)).collect();

    for (idx, ((start, end), (&pos, token))) in positions.iter().zip(
        pos_tags.iter().zip(tokens.iter())
    ).enumerate() {
        // Register markers
        if matches!(token.as_str(), "请" | "麻烦" | "求" | "麻烦你" | "请帮忙" | "拜托") {
            slots.push(Slot::new(SlotKind::Register, token.clone(), *start, *end)
                .with_sub_tag("politeness"));
            continue;
        }

        // Negation: 不/没/未/非 etc, or words starting with them
        if matches!(token.as_str(), "不" | "没" | "没有" | "未" | "非" | "无" | "勿" | "不要" | "无法" | "不能")
            || (token.starts_with("不") && token.chars().count() <= 3)
            || (token.starts_with("没") && token.chars().count() <= 3)
        {
            slots.push(Slot::new(SlotKind::Negation, token.clone(), *start, *end));
            continue;
        }

        // Conditional
        if matches!(token.as_str(), "如果" | "要是" | "假如" | "假设" | "若" | "倘若" | "假如说")
        {
            slots.push(Slot::new(SlotKind::Condition, token.clone(), *start, *end));
            continue;
        }

        // Q-type markers
        if is_qtype_marker(token) {
            slots.push(Slot::new(SlotKind::Qtype, token.clone(), *start, *end));
            continue;
        }

        // Action: verb early in sentence or followed by noun
        if pos == Pos::Verb && is_action_position(idx, n, &pos_tags) {
            slots.push(Slot::new(SlotKind::Action, token.clone(), *start, *end)
                .with_sub_tag("verb"));
            continue;
        }

        // Object: abstract noun
        if pos == Pos::Noun {
            // Check if this is a "scope-modified" object (previous token is scope modifier)
            if idx > 0 && is_scope_modifier(&tokens[idx - 1]) {
                let (prev_start, _) = positions[idx - 1];
                let merged = format!("{}{}", tokens[idx - 1], token);
                slots.retain(|s| s.char_start != prev_start);
                slots.push(Slot::new(SlotKind::Scope, merged, prev_start, *end)
                    .with_sub_tag("scope-noun"));
            } else {
                slots.push(Slot::new(SlotKind::Object, token.clone(), *start, *end)
                    .with_sub_tag("noun"));
            }
            continue;
        }

        // Scope-only modifier without following noun
        if is_scope_modifier(token) && (idx + 1 >= n || pos_tags[idx + 1] != Pos::Noun) {
            slots.push(Slot::new(SlotKind::Scope, token.clone(), *start, *end));
            continue;
        }

        // Adjective / modifier
        if pos == Pos::Adjective {
            slots.push(Slot::new(SlotKind::Modifier, token.clone(), *start, *end)
                .with_sub_tag("adj"));
            continue;
        }
    }

    // Post-process: merge adjacent Object slots into phrases
    merge_adjacent_objects(&mut slots);

    slots
}

/// Walk through the text and tokens simultaneously, computing (start, end)
/// char offsets for each token. Skips whitespace and punctuation.
fn compute_token_positions(text: &str, tokens: &[String]) -> Vec<(usize, usize)> {
    let text_chars: Vec<char> = text.chars().collect();
    let mut out = Vec::with_capacity(tokens.len());
    let mut ti = 0_usize; // text char index
    let mut wi = 0_usize; // word index

    while wi < tokens.len() && ti < text_chars.len() {
        // Skip whitespace and punctuation
        while ti < text_chars.len()
            && (text_chars[ti].is_whitespace()
                || is_punctuation(text_chars[ti]))
        {
            ti += 1;
        }
        if ti >= text_chars.len() { break; }

        // Try to match current word
        let word = &tokens[wi];
        let word_chars: Vec<char> = word.chars().collect();
        let wlen = word_chars.len();

        if ti + wlen <= text_chars.len() {
            let slice = &text_chars[ti..ti + wlen];
            if slice == word_chars.as_slice() {
                out.push((ti, ti + wlen));
                ti += wlen;
                wi += 1;
                continue;
            }
        }

        // Word doesn't match at expected position. Try single-char alignment.
        // This handles cases where the Viterbi output is a single character
        // that matches the current text position.
        if wlen == 1 {
            if text_chars[ti] == word_chars[0] {
                out.push((ti, ti + 1));
                ti += 1;
                wi += 1;
                continue;
            }
        }

        // Mismatch: skip one char of text and re-try on next iteration
        ti += 1;
    }
    out
}

fn is_punctuation(c: char) -> bool {
    c.is_ascii_punctuation()
        || matches!(c as u32,
            0x3000..=0x303F | // CJK Symbols and Punctuation
            0xFF00..=0xFFEF   // Halfwidth and Fullwidth Forms
        )
}

fn is_qtype_marker(token: &str) -> bool {
    matches!(token,
        "如何" | "怎么" | "怎样" | "怎么样" |
        "为什么" | "为何" |
        "是什么" | "什么是" |
        "哪些" | "什么" |
        "影响" | "机制" | "过程" | "原理" | "原因" | "方法" | "步骤" | "流程" |
        "区别" | "差异" | "对比" | "比较"
    ) || token.ends_with("机制") || token.ends_with("过程") || token.ends_with("原理")
}

fn is_action_position(idx: usize, n: usize, pos_tags: &[Pos]) -> bool {
    if idx <= n / 3 { return true; }
    if idx + 1 < n && pos_tags[idx + 1] == Pos::Noun { return true; }
    false
}

fn is_scope_modifier(token: &str) -> bool {
    matches!(token,
        "整体" | "部分" | "全部" | "总体" | "宏观" | "微观" | "全球" | "国内" | "国际" |
        "长期" | "短期" | "中期" | "当前" | "目前" | "现在" | "过去" | "未来"
    )
}

fn merge_adjacent_objects(slots: &mut Vec<Slot>) {
    let mut i = 0;
    while i < slots.len() {
        if slots[i].kind == SlotKind::Object {
            let mut j = i + 1;
            while j < slots.len()
                && slots[j].kind == SlotKind::Object
                && slots[j].char_start == slots[j - 1].char_end
            {
                j += 1;
            }
            if j > i + 1 {
                let start = slots[i].char_start;
                let end = slots[j - 1].char_end;
                let merged: String = slots[i..j].iter()
                    .map(|s| s.text.as_str())
                    .collect();
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
    fn test_finance_policy_prompt() {
        let text = "请阐述财政政策调整对宏观经济均衡状态的影响机制";
        let slots = extract_slots_zh(text);
        println!("Slots for '{}':", text);
        for s in &slots {
            println!("  {:?} '{}' [{}..{}]", s.kind, s.text, s.char_start, s.char_end);
        }
        let kinds: Vec<&'static str> = slots.iter().map(|s| s.kind.tag()).collect();
        assert!(kinds.contains(&"register"), "Should detect 请 as Register, got {:?}", kinds);
        assert!(kinds.contains(&"action"), "Should detect 阐述 as Action, got {:?}", kinds);
        assert!(kinds.contains(&"object"), "Should detect 财政政策 as Object, got {:?}", kinds);
        assert!(kinds.contains(&"qtype"), "Should detect 影响机制 as Qtype, got {:?}", kinds);

        let obj = slots.iter().find(|s| s.kind == SlotKind::Object).unwrap();
        assert!(obj.text.contains("财政政策"), "Object should contain 财政政策, got {}", obj.text);
    }

    #[test]
    fn test_runaway_paraphrase_blocked() {
        let original = "请阐述财政政策调整对宏观经济均衡状态的影响机制";
        let bad = "请说明国税规定变化对整体经济平衡的影响机制";

        let orig_slots = extract_slots_zh(original);
        let bad_slots = extract_slots_zh(bad);

        let orig_obj: Vec<&str> = orig_slots.iter()
            .filter(|s| s.kind == SlotKind::Object)
            .map(|s| s.text.as_str())
            .collect();
        let bad_obj: Vec<&str> = bad_slots.iter()
            .filter(|s| s.kind == SlotKind::Object)
            .map(|s| s.text.as_str())
            .collect();

        println!("Original objects: {:?}", orig_obj);
        println!("Bad rewrite objects: {:?}", bad_obj);

        assert!(orig_obj.iter().any(|o| o.contains("财政政策")),
                "Original should have 财政政策 Object");
        assert!(bad_obj.iter().any(|o| o.contains("国税规定")),
                "Bad should have 国税规定 Object");
    }

    #[test]
    fn test_qtype_change_caught() {
        let original = "请阐述财政政策对经济的影响机制";
        let bad = "请阐述财政政策对经济的影响过程";

        let orig = extract_slots_zh(original);
        let bad_slots = extract_slots_zh(bad);

        let orig_qt: Vec<&str> = orig.iter()
            .filter(|s| s.kind == SlotKind::Qtype)
            .map(|s| s.text.as_str())
            .collect();
        let bad_qt: Vec<&str> = bad_slots.iter()
            .filter(|s| s.kind == SlotKind::Qtype)
            .map(|s| s.text.as_str())
            .collect();

        assert!(orig_qt.iter().any(|q| q.contains("机制")),
                "Original Qtype should contain 机制, got {:?}", orig_qt);
        assert!(bad_qt.iter().any(|q| q.contains("过程")),
                "Bad Qtype should contain 过程, got {:?}", bad_qt);
    }

    #[test]
    fn test_register_change_caught() {
        let formal = "请说明这个方法";
        let casual = "说说这个方法";

        let f_slots = extract_slots_zh(formal);
        let c_slots = extract_slots_zh(casual);

        let f_has_register = f_slots.iter().any(|s| s.kind == SlotKind::Register);
        let c_has_register = c_slots.iter().any(|s| s.kind == SlotKind::Register);

        assert!(f_has_register, "请 should be detected as Register");
        assert!(!c_has_register, "casual 说说 should NOT have Register slot");
    }

    #[test]
    fn test_negation_caught() {
        let s = extract_slots_zh("不要使用这种方法");
        let has_neg = s.iter().any(|x| x.kind == SlotKind::Negation);
        assert!(has_neg, "Should detect 不要 as Negation, got: {:?}", s);
    }

    #[test]
    fn test_conditional_caught() {
        let text = "如果你有时间，请告诉我";
        let s = extract_slots_zh(text);
        let toks = frequency::tokenize_cn(text);
        println!("=== Conditional test ===");
        println!("Tokens: {:?}", toks);
        for x in &s {
            println!("  slot: {:?} '{}' [{}..{}]", x.kind, x.text, x.char_start, x.char_end);
        }
        let has_cond = s.iter().any(|x| x.kind == SlotKind::Condition);
        assert!(has_cond, "Should detect 如果 as Condition");
    }

    #[test]
    fn test_scope_modifier_merge() {
        let s = extract_slots_zh("分析宏观经济趋势");
        let scope = s.iter().find(|x| x.kind == SlotKind::Scope);
        assert!(scope.is_some(), "Should detect 宏观经济 as Scope, got: {:?}", s);
        if let Some(sc) = scope {
            assert!(sc.text.contains("宏观经济"), "Got: {}", sc.text);
        }
    }

    #[test]
    fn test_empty() {
        let s = extract_slots_zh("");
        assert!(s.is_empty());
    }

    #[test]
    fn test_simple_subject() {
        let s = extract_slots_zh("我是一个学生");
        let obj = s.iter().find(|x| x.kind == SlotKind::Object);
        assert!(obj.is_some(), "Should detect 学生 as Object, got: {:?}", s);
    }
}
