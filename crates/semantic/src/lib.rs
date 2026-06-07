//! FreqPrompt v3 — Semantic Guard
//!
//! Slot-based semantic preservation layer. This crate extracts semantic
//! roles (slots) from prompts and exposes primitives for verifying that
//! a paraphrase preserves the critical slots.
//!
//! ## Why slots?
//!
//! Pure Zipf-frequency optimization can silently shift meaning. For example:
//!   - 阐述 → 说说       (REGISTER/ACTION change)
//!   - 财政政策 → 国税规定  (OBJECT narrowing — parent → child)
//!   - 影响机制 → 过程     (QTYPE change — mechanism vs process)
//!   - 宏观经济 → 整体经济  (SCOPE change)
//!
//! This crate identifies the **semantic roles** that must be preserved and
//! gives downstream layers (TypeScript via WASM) the structure needed to
//! run per-slot verification.
//!
//! ## Module structure
//!
//! - [`slot`] — slot type definitions and core data structures
//! - [`pos`] — POS tagging for Chinese and English (lightweight, rule-based)
//! - [`extract_zh`] — Chinese slot extractor (uses frequency crate's Viterbi)
//! - [`extract_en`] — English slot extractor (rule-based)
//! - [`placeholders`] — placeholder/entity detection (must match exactly)
//! - [`verify`] — slot-presence verification (does the candidate have the same slots?)

#![doc(html_root_url = "https://docs.rs/semantic/0.1.0")]

pub mod slot;
pub mod pos;
pub mod extract_zh;
pub mod extract_en;
pub mod placeholders;
pub mod verify;

pub use slot::{Slot, SlotKind, SlotVerdict, SlotThresholds};
pub use pos::{Pos, tag_zh, tag_en};
pub use extract_zh::extract_slots_zh;
pub use extract_en::extract_slots_en;
pub use placeholders::{PlaceholderKind, detect_placeholders};
pub use verify::{aggregate_preservation, apply_thresholds, exact_match_check, structural_check};

/// Detect language and dispatch to the right extractor.
///
/// `lang` accepts:
///   - `"zh"` — Chinese (CJK ratio > 30%)
///   - `"en"` — English (default)
///   - `"auto"` — auto-detect from content
///
/// # Examples
///
/// ```
/// # use semantic::extract_slots;
/// let slots = extract_slots("今天天气如何？", "zh");
/// assert!(!slots.is_empty());
///
/// let slots_en = extract_slots("How is the weather?", "en");
/// assert!(!slots_en.is_empty());
/// ```
pub fn extract_slots(text: &str, lang: &str) -> Vec<Slot> {
    let effective_lang = if lang == "auto" {
        if is_chinese(text) { "zh" } else { "en" }
    } else {
        lang
    };

    let placeholders = placeholders::detect_placeholders(text);
    let semantic_slots = match effective_lang {
        "zh" => extract_zh::extract_slots_zh(text),
        _ => extract_en::extract_slots_en(text),
    };

    let mut all = placeholders;
    all.extend(semantic_slots);
    // Sort by character position for stable downstream processing
    all.sort_by_key(|s| s.char_start);
    all
}

fn is_chinese(text: &str) -> bool {
    let total = text.chars().filter(|c| !c.is_whitespace()).count();
    if total == 0 { return false; }
    let cjk = text.chars().filter(|c| {
        matches!(*c as u32,
            0x4E00..=0x9FFF | 0x3400..=0x4DBF
            | 0xF900..=0xFAFF | 0x3000..=0x303F
        )
    }).count();
    (cjk as f64 / total as f64) > 0.3
}
