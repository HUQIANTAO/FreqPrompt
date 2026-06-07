//! Lightweight POS tagging for Chinese and English.
//!
//! For Chinese, we use lexical heuristics based on character patterns
//! and known-word lists. For English, we combine suffix patterns with
//! small known-words lists for common irregular forms.

use serde::{Deserialize, Serialize};

/// A coarse POS tag sufficient for slot extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pos {
    /// Function words (的, 了, 在, of, the)
    Function,
    /// Verbs (阐述, 说明, run, explain)
    Verb,
    /// Nouns (政策, 影响, policy, effect)
    Noun,
    /// Adjectives (整体, 详细, careful, big)
    Adjective,
    /// Adverbs (请, 详细地, carefully)
    Adverb,
    /// Pronouns / determiners (我, 你, this, that)
    Pronoun,
    /// Prepositions / conjunctions (在, 和, of, and)
    Preposition,
    /// Numbers, dates, entities
    Entity,
    /// Punctuation / unknown
    Other,
}

impl Pos {
    pub fn is_content_word(self) -> bool {
        matches!(self, Pos::Verb | Pos::Noun | Pos::Adjective | Pos::Adverb)
    }
}

/// Known English verbs (irregular forms not caught by suffix rules).
const EN_KNOWN_VERBS: &[&str] = &[
    "explain", "examine", "describe", "analyze", "analyse", "modify", "discuss",
    "elaborate", "list", "enumerate", "compare", "contrast",
    "investigate", "study", "research", "review", "summarize", "outline",
    "clarify", "define", "identify", "evaluate", "assess", "interpret",
    "predict", "recommend", "suggest", "propose", "argue", "claim", "state",
    "report", "tell", "say", "speak", "talk", "write", "read", "understand",
    "know", "think", "consider", "feel", "want", "need", "use", "make", "do",
    "go", "come", "take", "give", "get", "see", "look", "find", "show", "help",
    "include", "contain", "involve", "require", "depend", "vary", "differ",
    "express", "indicate", "reflect", "represent",
    "highlight", "emphasize", "underline", "demonstrate", "illustrate",
    "is", "are", "was", "were", "been",
];

/// Known English adjectives not caught by suffix rules.
const EN_KNOWN_ADJECTIVES: &[&str] = &[
    "global", "national", "local", "regional", "federal", "central",
    "macroeconomic", "microeconomic", "monetary", "fiscal", "financial",
    "economic", "structural", "industrial", "commercial",
    "overall", "entire", "whole", "complete", "partial", "full", "empty",
    "main", "major", "minor", "key", "primary", "secondary", "tertiary",
    "important", "critical", "essential", "necessary", "crucial", "vital",
    "obvious", "clear", "unclear", "ambiguous",
    "current", "present", "past", "future", "recent", "ancient", "modern",
];

/// Tag a Chinese word (1-4 chars) with a coarse POS.
pub fn tag_zh(word: &str) -> Pos {
    if word.is_empty() { return Pos::Other; }
    if word.chars().count() == 1 {
        return tag_zh_single_char(word);
    }
    tag_zh_multi_char(word)
}

fn tag_zh_single_char(word: &str) -> Pos {
    let c = word.chars().next().unwrap();
    match c {
        '的' | '了' | '着' | '过' | '们' | '吗' | '呢' | '吧' | '啊' | '呀' | '哦' | '嘛' => Pos::Function,
        '不' | '没' | '非' | '无' | '未' | '勿' => Pos::Adverb,
        '在' | '从' | '向' | '往' | '到' | '给' | '为' | '对' | '以' | '把' | '被' => Pos::Preposition,
        '我' | '你' | '他' | '她' | '它' | '这' | '那' | '此' => Pos::Pronoun,
        '和' | '与' | '或' | '及' | '而' | '但' | '因' | '所' => Pos::Function,
        '是' | '有' | '做' | '说' | '看' | '想' | '去' | '来' | '走' | '写' | '读' | '听' | '会' | '能' | '将' => Pos::Verb,
        '请' | '求' => Pos::Adverb,
        '一' | '二' | '三' | '四' | '五' | '六' | '七' | '八' | '九' | '十' | '百' | '千' | '万' | '亿' | '几' | '多' | '少' => Pos::Entity,
        _ => Pos::Noun,
    }
}

fn tag_zh_multi_char(word: &str) -> Pos {
    // Negation patterns first (most critical for slot preservation)
    if word.starts_with("不") || word.starts_with("没") || word.starts_with("未") {
        return Pos::Adverb;
    }

    // Verb indicators
    let verb_words = ["阐述", "说明", "解释", "介绍", "描述", "讲述", "讲",
                     "分析", "论述", "讨论", "研究", "调查",
                     "请", "求", "帮忙", "帮", "试试",
                     "进行", "实施", "执行", "开展", "做出", "实行",
                     "影响", "导致", "造成", "引起", "带来", "产生", "推动",
                     "发生", "出现", "存在", "具有", "包含", "涉及"];
    if verb_words.contains(&word) { return Pos::Verb; }

    let verb_suffixes = ["做了", "去过", "看过", "读过", "听过", "写过", "想过", "说过",
                         "起来", "下去", "上来", "下来"];
    for s in &verb_suffixes {
        if word.ends_with(s) { return Pos::Verb; }
    }

    // Adjective words
    let adj_words = ["整体", "部分", "全部", "主要", "次要", "核心", "基本",
                     "重要", "关键", "详细", "具体", "完整", "全面", "系统", "深入",
                     "明显", "显著", "有效", "无效", "成功", "失败",
                     "好", "坏", "新", "旧", "对", "错"];
    if adj_words.contains(&word) { return Pos::Adjective; }

    // Noun indicators (most words fall here)
    let noun_words = ["财政政策", "货币政策", "宏观经济", "微观经济", "经济", "金融",
                      "机制", "过程", "方法", "方式", "路径", "原理", "原因", "步骤",
                      "国家", "政府", "社会", "市场", "企业", "公众", "消费者",
                      "调整", "改革", "转变", "变化", "影响", "作用", "效应", "关系",
                      "系统", "结构", "功能", "特征", "性质", "状态", "情况", "情形",
                      "分析", "研究", "调查", "报告", "论文", "文章", "理论", "概念",
                      "观点", "思想", "理念", "原则", "工具", "手段",
                      "均衡", "平衡", "稳定", "增长", "发展", "政策", "国税", "地税"];
    if noun_words.contains(&word) { return Pos::Noun; }

    let noun_suffixes = ["政策", "机制", "影响", "作用", "效应", "关系",
                         "系统", "结构", "功能", "特征", "性质", "状态",
                         "过程", "方法", "方式", "路径", "手段", "工具",
                         "理论", "概念", "观点", "思想", "理念", "原则",
                         "分析", "研究", "调查", "报告", "论文", "文章",
                         "经济", "财政", "货币", "金融", "市场", "企业",
                         "国家", "政府", "社会", "公众", "消费者",
                         "均衡", "平衡", "稳定", "增长", "发展", "变化",
                         "调整", "改革", "转变", "变化", "改动", "修改"];
    for s in &noun_suffixes {
        if word.ends_with(s) { return Pos::Noun; }
    }

    if word.ends_with("地") { return Pos::Adverb; }

    // Default: noun
    Pos::Noun
}

/// Tag an English word with a coarse POS.
pub fn tag_en(word: &str) -> Pos {
    let lower = word.to_lowercase();
    if lower.is_empty() { return Pos::Other; }

    // Function / stop words
    if matches!(lower.as_str(),
        "a" | "an" | "the" | "of" | "in" | "on" | "at" | "to" | "for" | "by" | "with" |
        "from" | "as" | "is" | "are" | "was" | "were" | "be" | "been" | "being" |
        "am" | "ai" |
        "and" | "or" | "but" | "if" | "then" | "so" | "than" | "that" | "this" | "these" | "those" |
        "it" | "its" | "he" | "she" | "they" | "them" | "his" | "her" | "their" |
        "i" | "you" | "we" | "us" | "me" | "my" | "your" | "our" |
        "do" | "does" | "did" | "done" |
        "have" | "has" | "had" | "having" |
        "will" | "would" | "should" | "could" | "can" | "may" | "might" | "must" | "shall" |
        "not" | "no" | "yes"
    ) {
        return Pos::Function;
    }

    // Preposition / conjunction
    if matches!(lower.as_str(),
        "about" | "above" | "across" | "after" | "against" | "along" | "among" | "around" |
        "before" | "behind" | "below" | "beneath" | "beside" | "between" | "beyond" |
        "during" | "except" | "inside" | "into" | "near" | "onto" | "opposite" | "outside" |
        "over" | "through" | "toward" | "under" | "underneath" | "until" | "upon" | "within" | "without" |
        "although" | "because" | "since" | "unless" | "when" | "whenever" | "where" | "whereas" | "wherever" | "whether" | "while"
    ) {
        return Pos::Preposition;
    }

    // Pronoun
    if matches!(lower.as_str(),
        "i" | "me" | "my" | "mine" | "myself" |
        "you" | "your" | "yours" | "yourself" |
        "he" | "him" | "his" | "himself" |
        "she" | "her" | "hers" | "herself" |
        "it" | "its" | "itself" |
        "we" | "us" | "our" | "ours" | "ourselves" |
        "they" | "them" | "their" | "theirs" | "themselves" |
        "this" | "that" | "these" | "those" | "who" | "what" | "which" | "whom"
    ) {
        return Pos::Pronoun;
    }

    // Known-words overrides (irregular forms not caught by suffix rules)
    if EN_KNOWN_VERBS.contains(&lower.as_str()) {
        return Pos::Verb;
    }
    if EN_KNOWN_ADJECTIVES.contains(&lower.as_str()) {
        return Pos::Adjective;
    }

    // Suffix-based heuristics
    // Adverb first (before noun "-er" rule catches "career" wrongly)
    if lower.ends_with("ly") && lower.len() > 4 { return Pos::Adverb; }

    // Verb suffixes
    if lower.ends_with("ize") || lower.ends_with("ise") || lower.ends_with("ify")
        || lower.ends_with("ate") || lower.ends_with("ect")
    {
        return Pos::Verb;
    }

    // Noun suffixes
    if lower.ends_with("tion") || lower.ends_with("sion") || lower.ends_with("ment")
        || lower.ends_with("ity") || lower.ends_with("ness") || lower.ends_with("ence")
        || lower.ends_with("ance") || lower.ends_with("ist") || lower.ends_with("ism")
        || lower.ends_with("ology") || lower.ends_with("graphy") || lower.ends_with("ics")
    {
        return Pos::Noun;
    }

    // Noun "-er" / "-or" (only when not in verb form)
    if lower.ends_with("er") || lower.ends_with("or") {
        return Pos::Noun;
    }

    // Adjective "-ed" past participle as adjective (careful: also matches verbs)
    // Heuristic: if word ends in "-ed" and isn't in known verb forms, treat as adj
    if lower.ends_with("ed") && lower.len() > 4 && !EN_KNOWN_VERBS.contains(&lower.as_str()) {
        return Pos::Adjective;
    }

    // Adjective suffixes
    if lower.ends_with("al") || lower.ends_with("ous") || lower.ends_with("ive")
        || lower.ends_with("ful") || lower.ends_with("less") || lower.ends_with("able")
        || lower.ends_with("ible") || lower.ends_with("ic") || lower.ends_with("ical")
    {
        return Pos::Adjective;
    }

    // Default
    Pos::Noun
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zh_verb_detection() {
        assert_eq!(tag_zh("阐述"), Pos::Verb);
        assert_eq!(tag_zh("说明"), Pos::Verb);
        assert_eq!(tag_zh("解释"), Pos::Verb);
        assert_eq!(tag_zh("分析"), Pos::Verb);
    }

    #[test]
    fn test_zh_noun_detection() {
        assert_eq!(tag_zh("财政政策"), Pos::Noun);
        assert_eq!(tag_zh("宏观经济"), Pos::Noun);
        assert_eq!(tag_zh("均衡状态"), Pos::Noun);
        assert_eq!(tag_zh("影响机制"), Pos::Noun);
    }

    #[test]
    fn test_zh_function_words() {
        assert_eq!(tag_zh("的"), Pos::Function);
        assert_eq!(tag_zh("了"), Pos::Function);
        assert_eq!(tag_zh("在"), Pos::Preposition);
    }

    #[test]
    fn test_zh_negation() {
        assert_eq!(tag_zh("不"), Pos::Adverb);
        assert_eq!(tag_zh("没"), Pos::Adverb);
        assert_eq!(tag_zh("不要"), Pos::Adverb);
    }

    #[test]
    fn test_zh_register_marker() {
        assert_eq!(tag_zh("请"), Pos::Adverb);
    }

    #[test]
    fn test_zh_verb_particle() {
        assert_eq!(tag_zh("做了"), Pos::Verb);
        assert_eq!(tag_zh("去过"), Pos::Verb);
    }

    #[test]
    fn test_en_function() {
        assert_eq!(tag_en("the"), Pos::Function);
        assert_eq!(tag_en("of"), Pos::Function);
        assert_eq!(tag_en("is"), Pos::Function);
    }

    #[test]
    fn test_en_suffix_noun() {
        assert_eq!(tag_en("policy"), Pos::Noun);
        assert_eq!(tag_en("mechanism"), Pos::Noun);
        assert_eq!(tag_en("adjustment"), Pos::Noun);
        assert_eq!(tag_en("economy"), Pos::Noun);
    }

    #[test]
    fn test_en_known_verb() {
        assert_eq!(tag_en("explain"), Pos::Verb);
        assert_eq!(tag_en("analyze"), Pos::Verb);
        assert_eq!(tag_en("modify"), Pos::Verb);
        assert_eq!(tag_en("describe"), Pos::Verb);
        assert_eq!(tag_en("examine"), Pos::Verb);
    }

    #[test]
    fn test_en_suffix_verb() {
        assert_eq!(tag_en("realize"), Pos::Verb);
        assert_eq!(tag_en("modify"), Pos::Verb);  // also in known list
    }

    #[test]
    fn test_en_suffix_adverb() {
        assert_eq!(tag_en("carefully"), Pos::Adverb);
        assert_eq!(tag_en("clearly"), Pos::Adverb);
    }

    #[test]
    fn test_en_suffix_adj() {
        assert_eq!(tag_en("careful"), Pos::Adjective);
        assert_eq!(tag_en("detailed"), Pos::Adjective);
        assert_eq!(tag_en("global"), Pos::Adjective);
    }

    #[test]
    fn test_en_known_adj() {
        assert_eq!(tag_en("macroeconomic"), Pos::Adjective);
        assert_eq!(tag_en("fiscal"), Pos::Adjective);
        assert_eq!(tag_en("monetary"), Pos::Adjective);
    }

    #[test]
    fn test_en_preposition() {
        assert_eq!(tag_en("during"), Pos::Preposition);
        assert_eq!(tag_en("although"), Pos::Preposition);
    }
}
