//! Slot type definitions and core data structures.
//!
//! A *slot* is a span of text that carries a specific semantic role.
//! Paraphrasing must preserve all *critical* slots; failure to do so
//! is a meaning shift, even if Zipf frequency improves.

use serde::{Deserialize, Serialize};

/// The semantic role of a slot.
///
/// Criticality is implicit in the kind — see `SlotThresholds` for the
/// per-kind similarity thresholds used during verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotKind {
    /// Action / verb / verb-phrase (阐述, 说明, explain).
    /// Must remain the same type of action.
    Action,

    /// Object / main noun / noun-phrase (财政政策, fiscal policy).
    /// Must remain the same concept — narrowing or widening is forbidden.
    Object,

    /// Scope / range qualifier (宏观经济, 整体经济).
    /// Must not be widened or narrowed.
    Scope,

    /// Modifier / adjective / adverb (整体, 详细, carefully).
    /// Has more freedom to change.
    Modifier,

    /// Question type / answer-shape (影响机制, 过程, list).
    /// Changing this changes what the user is asking for.
    Qtype,

    /// Register / formality / politeness marker (请, 求, please).
    /// Should be roughly preserved (formal vs casual).
    Register,

    /// Placeholder / variable (e.g., {name}, [TASK], `<email>`).
    /// **MUST** be preserved exactly. Same character, same position.
    Placeholder,

    /// Named entity / number / date / URL / email.
    /// Must be preserved with high fidelity.
    Entity,

    /// Negation marker (不, 没有, not, never).
    /// **MUST** be preserved exactly. Dropping negation flips meaning.
    Negation,

    /// Conditional / temporal clause (如果, when, if, after).
    /// Should be preserved.
    Condition,
}

impl SlotKind {
    /// Human-readable name for UI display.
    pub fn display_name(self) -> &'static str {
        match self {
            SlotKind::Action => "动作 (Action)",
            SlotKind::Object => "对象 (Object)",
            SlotKind::Scope => "范围 (Scope)",
            SlotKind::Modifier => "修饰 (Modifier)",
            SlotKind::Qtype => "问法 (Q-Type)",
            SlotKind::Register => "语体 (Register)",
            SlotKind::Placeholder => "占位符 (Placeholder)",
            SlotKind::Entity => "实体 (Entity)",
            SlotKind::Negation => "否定 (Negation)",
            SlotKind::Condition => "条件 (Condition)",
        }
    }

    /// Short tag for CSS class / data attribute.
    pub fn tag(self) -> &'static str {
        match self {
            SlotKind::Action => "action",
            SlotKind::Object => "object",
            SlotKind::Scope => "scope",
            SlotKind::Modifier => "modifier",
            SlotKind::Qtype => "qtype",
            SlotKind::Register => "register",
            SlotKind::Placeholder => "placeholder",
            SlotKind::Entity => "entity",
            SlotKind::Negation => "negation",
            SlotKind::Condition => "condition",
        }
    }
}

/// A single slot extracted from a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    /// The semantic role.
    pub kind: SlotKind,
    /// The surface text of the slot.
    pub text: String,
    /// Character offset start (inclusive) into the original prompt.
    pub char_start: usize,
    /// Character offset end (exclusive).
    pub char_end: usize,
    /// Optional sub-classification (e.g., "verb", "noun" within Action).
    pub sub_tag: Option<String>,
}

impl Slot {
    pub fn new(kind: SlotKind, text: impl Into<String>, char_start: usize, char_end: usize) -> Self {
        Self {
            kind,
            text: text.into(),
            char_start,
            char_end,
            sub_tag: None,
        }
    }

    pub fn with_sub_tag(mut self, tag: impl Into<String>) -> Self {
        self.sub_tag = Some(tag.into());
        self
    }

    /// Character length of the slot text.
    pub fn len_chars(&self) -> usize {
        self.text.chars().count()
    }

    /// Whether this slot kind requires exact match (no embedding similarity).
    pub fn requires_exact_match(&self) -> bool {
        matches!(
            self.kind,
            SlotKind::Placeholder | SlotKind::Negation
        )
    }
}

/// Outcome of a per-slot verification between original and candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotVerdict {
    /// The original slot.
    pub original: Slot,
    /// The matched candidate slot text, or None if no match.
    pub matched: Option<String>,
    /// Cosine similarity (0.0–1.0); 1.0 for exact matches.
    pub similarity: f64,
    /// Whether the slot passes the threshold.
    pub passes: bool,
    /// Threshold that was applied.
    pub threshold: f64,
}

/// Per-slot-kind similarity thresholds.
///
/// These are conservative defaults — they can be loosened or tightened
/// by the user via the UI. The defaults are calibrated so that:
///   - Mechanism ↔ Process (Qtype) is rejected (0.95 threshold)
///   - 财政政策 ↔ 国税规定 (Object) is rejected (0.90 threshold)
///   - 请 ↔ (no marker) (Register) is borderline
///   - 整体 ↔ 完全 (Modifier) passes (0.75 threshold)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotThresholds {
    pub action: f64,
    pub object: f64,
    pub scope: f64,
    pub modifier: f64,
    pub qtype: f64,
    pub register: f64,
    pub entity: f64,
    pub condition: f64,
}

impl Default for SlotThresholds {
    fn default() -> Self {
        Self {
            action: 0.85,
            object: 0.90,
            scope: 0.95,
            modifier: 0.75,
            qtype: 0.95,
            register: 0.80,
            entity: 0.95,
            condition: 0.90,
        }
    }
}

impl SlotThresholds {
    /// Get the threshold for a given slot kind.
    pub fn for_kind(&self, kind: SlotKind) -> f64 {
        match kind {
            SlotKind::Action => self.action,
            SlotKind::Object => self.object,
            SlotKind::Scope => self.scope,
            SlotKind::Modifier => self.modifier,
            SlotKind::Qtype => self.qtype,
            SlotKind::Register => self.register,
            SlotKind::Placeholder => 1.0,   // exact
            SlotKind::Entity => self.entity,
            SlotKind::Negation => 1.0,       // exact
            SlotKind::Condition => self.condition,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_kind_display() {
        assert_eq!(SlotKind::Action.display_name(), "动作 (Action)");
        assert_eq!(SlotKind::Qtype.display_name(), "问法 (Q-Type)");
    }

    #[test]
    fn test_slot_kind_tag() {
        assert_eq!(SlotKind::Object.tag(), "object");
    }

    #[test]
    fn test_slot_requires_exact_match() {
        let p = Slot::new(SlotKind::Placeholder, "{name}", 0, 6);
        let a = Slot::new(SlotKind::Action, "explain", 0, 7);
        assert!(p.requires_exact_match());
        assert!(!a.requires_exact_match());
    }

    #[test]
    fn test_thresholds_default() {
        let t = SlotThresholds::default();
        assert!(t.object > 0.85);
        assert!(t.qtype > 0.90);
        assert!(t.modifier < 0.80);
    }

    #[test]
    fn test_thresholds_for_kind() {
        let t = SlotThresholds::default();
        assert_eq!(t.for_kind(SlotKind::Placeholder), 1.0);
        assert_eq!(t.for_kind(SlotKind::Negation), 1.0);
        assert_eq!(t.for_kind(SlotKind::Object), 0.90);
    }
}
