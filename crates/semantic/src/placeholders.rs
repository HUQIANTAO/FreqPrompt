//! Placeholder and entity detection.
//!
//! These slots require **exact** match — embedding similarity is bypassed.
//! Examples:
//!   - `{name}`, `[TASK]`, `<email>`, `$VAR`
//!   - Numbers, percentages, currencies
//!   - URLs, emails, dates
//!   - Quoted strings

use serde::{Deserialize, Serialize};

use crate::slot::{Slot, SlotKind};

/// Category of placeholder detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceholderKind {
    /// `{name}`, `[TASK]`, `<email>`, `$VAR`
    Bracket,
    /// Quoted string "..." or '...'
    Quoted,
    /// Pure number or numeric expression (123, 3.14, 1e5)
    Number,
    /// Percentage, currency, or unit-attached number
    Quantity,
    /// URL or email address
    Reference,
    /// ISO date YYYY-MM-DD or YYYY年MM月DD日
    Date,
    /// CJK date format (年月日, 号)
    DateCn,
}

/// Detect all placeholders / entities in a text.
///
/// Order matters: longer/more specific patterns first.
pub fn detect_placeholders(text: &str) -> Vec<Slot> {
    let mut slots = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();

    let mut byte_pos = 0;

    // Priority: brackets > quoted > URL > email > ISO date > CJK date > number
    while byte_pos < n {
        let c = chars[byte_pos];

        // { } brackets
        if c == '{' {
            if let Some(end) = find_matching_bracket(&chars, byte_pos, '{', '}') {
                let text_slot: String = chars[byte_pos..=end].iter().collect();
                slots.push(Slot::new(
                    SlotKind::Placeholder,
                    text_slot,
                    byte_pos,
                    end + 1,
                ).with_sub_tag("braces"));
                byte_pos = end + 1;
                continue;
            }
        }
        // [ ] brackets
        if c == '[' {
            if let Some(end) = find_matching_bracket(&chars, byte_pos, '[', ']') {
                let text_slot: String = chars[byte_pos..=end].iter().collect();
                slots.push(Slot::new(
                    SlotKind::Placeholder,
                    text_slot,
                    byte_pos,
                    end + 1,
                ).with_sub_tag("square"));
                byte_pos = end + 1;
                continue;
            }
        }
        // < > brackets (XML-like)
        if c == '<' {
            if let Some(end) = find_matching_bracket(&chars, byte_pos, '<', '>') {
                let text_slot: String = chars[byte_pos..=end].iter().collect();
                // Only treat as placeholder if it looks like a tag, not comparison
                if is_placeholder_like(&text_slot) {
                    slots.push(Slot::new(
                        SlotKind::Placeholder,
                        text_slot,
                        byte_pos,
                        end + 1,
                    ).with_sub_tag("angle"));
                    byte_pos = end + 1;
                    continue;
                }
            }
        }
        // $VAR (shell-style)
        if c == '$' && byte_pos + 1 < n && (chars[byte_pos + 1].is_ascii_alphabetic() || chars[byte_pos + 1] == '_') {
            let mut end = byte_pos + 1;
            while end < n && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            let text_slot: String = chars[byte_pos..end].iter().collect();
            slots.push(Slot::new(
                SlotKind::Placeholder,
                text_slot,
                byte_pos,
                end,
            ).with_sub_tag("shell"));
            byte_pos = end;
            continue;
        }

        // Quoted strings
        if c == '"' || c == '\'' || c == '\u{2018}' || c == '\u{2019}' {
            let close = match c {
                '"' => '"',
                '\'' => '\'',
                _ => c,
            };
            if let Some(end) = find_close_quote(&chars, byte_pos + 1, close) {
                let text_slot: String = chars[byte_pos..=end].iter().collect();
                slots.push(Slot::new(
                    SlotKind::Placeholder,
                    text_slot,
                    byte_pos,
                    end + 1,
                ).with_sub_tag("quoted"));
                byte_pos = end + 1;
                continue;
            }
        }

        // URLs
        if byte_pos + 4 <= n && &chars[byte_pos..byte_pos + 4] == ['h', 't', 't', 'p'] {
            if let Some(end) = find_url_end(&chars, byte_pos) {
                let text_slot: String = chars[byte_pos..end].iter().collect();
                slots.push(Slot::new(
                    SlotKind::Entity,
                    text_slot,
                    byte_pos,
                    end,
                ).with_sub_tag("url"));
                byte_pos = end;
                continue;
            }
        }

        // Email (rough: word@word.word)
        if c.is_ascii_alphabetic() {
            if let Some(end) = find_email_end(&chars, byte_pos) {
                let text_slot: String = chars[byte_pos..end].iter().collect();
                if text_slot.contains('@') && text_slot.contains('.') {
                    slots.push(Slot::new(
                        SlotKind::Entity,
                        text_slot,
                        byte_pos,
                        end,
                    ).with_sub_tag("email"));
                    byte_pos = end;
                    continue;
                }
            }
        }

        // Numbers (with optional unit/percent/currency)
        if c.is_ascii_digit() || (c == '-' && byte_pos + 1 < n && chars[byte_pos + 1].is_ascii_digit()) {
            // First, try date patterns (longer/more specific)
            if c.is_ascii_digit() && byte_pos + 10 <= n {
                let candidate: String = chars[byte_pos..byte_pos + 10].iter().collect();
                if is_iso_date(&candidate) {
                    slots.push(Slot::new(
                        SlotKind::Entity,
                        candidate,
                        byte_pos,
                        byte_pos + 10,
                    ).with_sub_tag("iso-date"));
                    byte_pos += 10;
                    continue;
                }
            }
            if c.is_ascii_digit() {
                if let Some(end) = find_cn_date_end(&chars, byte_pos) {
                    let text_slot: String = chars[byte_pos..end].iter().collect();
                    slots.push(Slot::new(
                        SlotKind::Entity,
                        text_slot,
                        byte_pos,
                        end,
                    ).with_sub_tag("cn-date"));
                    byte_pos = end;
                    continue;
                }
            }
            // Plain number
            if let Some(end) = find_number_end(&chars, byte_pos) {
                let text_slot: String = chars[byte_pos..end].iter().collect();
                slots.push(Slot::new(
                    SlotKind::Entity,
                    text_slot,
                    byte_pos,
                    end,
                ).with_sub_tag("number"));
                byte_pos = end;
                continue;
            }
        }

        byte_pos += 1;
    }

    slots
}

fn find_matching_bracket(chars: &[char], start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0;
    for i in start..chars.len() {
        if chars[i] == open { depth += 1; }
        else if chars[i] == close {
            depth -= 1;
            if depth == 0 { return Some(i); }
        }
        // Don't span newlines
        if chars[i] == '\n' { return None; }
    }
    None
}

fn find_close_quote(chars: &[char], start: usize, close: char) -> Option<usize> {
    for i in start..chars.len() {
        if chars[i] == close { return Some(i); }
        if chars[i] == '\n' { return None; }
    }
    None
}

fn find_url_end(chars: &[char], start: usize) -> Option<usize> {
    let mut end = start;
    while end < chars.len() {
        let c = chars[end];
        if c.is_whitespace() || c == ')' || c == ']' || c == '>' || c == '<' || c == '"' {
            break;
        }
        end += 1;
    }
    if end > start { Some(end) } else { None }
}

fn find_email_end(chars: &[char], start: usize) -> Option<usize> {
    let mut end = start;
    while end < chars.len() {
        let c = chars[end];
        if c.is_alphanumeric() || c == '@' || c == '.' || c == '_' || c == '-' || c == '+' {
            end += 1;
        } else {
            break;
        }
    }
    if end > start { Some(end) } else { None }
}

fn find_number_end(chars: &[char], start: usize) -> Option<usize> {
    let mut end = start;
    let mut saw_digit = false;
    let mut saw_dot = false;
    while end < chars.len() {
        let c = chars[end];
        if c.is_ascii_digit() {
            saw_digit = true;
            end += 1;
        } else if c == '.' && !saw_dot && end + 1 < chars.len() && chars[end + 1].is_ascii_digit() {
            saw_dot = true;
            end += 1;
        } else if c == ',' && end + 1 < chars.len() && chars[end + 1].is_ascii_digit() {
            // 1,000 style
            end += 1;
        } else if c == 'e' && end + 1 < chars.len() && (chars[end + 1].is_ascii_digit() || chars[end + 1] == '-') {
            end += 1;
        } else if matches!(c, '%' | '$' | '€' | '¥' | '£' | '元' | '块' | '千' | '万' | '亿' | 'k' | 'K' | 'm' | 'M') {
            end += 1;
            break;
        } else {
            break;
        }
    }
    if saw_digit { Some(end) } else { None }
}

fn is_iso_date(s: &str) -> bool {
    // YYYY-MM-DD
    let bytes = s.as_bytes();
    if bytes.len() != 10 { return false; }
    bytes[0..4].iter().all(|b| b.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(|b| b.is_ascii_digit())
}

fn find_cn_date_end(chars: &[char], start: usize) -> Option<usize> {
    // Pattern: digits 年 digits 月 [digits 日]
    let mut end = start;
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    if end == start || end >= chars.len() || chars[end] != '年' {
        return None;
    }
    end += 1;
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    if end >= chars.len() || chars[end] != '月' {
        return None;
    }
    end += 1;
    let mut has_day = false;
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
        has_day = true;
    }
    if has_day && end < chars.len() && chars[end] == '日' {
        end += 1;
    }
    Some(end)
}

fn is_placeholder_like(s: &str) -> bool {
    // Reject HTML-like comparison ops (e.g., "5<x>10")
    if s.contains('=') || s.contains(' ') { return false; }
    if s.len() < 3 { return false; }
    // Reject "1<2" pattern
    if s.chars().filter(|c| c.is_ascii_digit()).count() > 0
        && s.chars().filter(|c| c.is_ascii_alphabetic()).count() == 0
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brace_placeholder() {
        let s = detect_placeholders("Hello {name}, welcome!");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].text, "{name}");
        assert_eq!(s[0].kind, SlotKind::Placeholder);
    }

    #[test]
    fn test_square_placeholder() {
        let s = detect_placeholders("Please do [TASK] carefully");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].text, "[TASK]");
    }

    #[test]
    fn test_shell_var() {
        let s = detect_placeholders("Set $HOME and $PATH first");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].text, "$HOME");
        assert_eq!(s[1].text, "$PATH");
    }

    #[test]
    fn test_quoted_string() {
        let s = detect_placeholders("He said \"hello world\" to me");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].text, "\"hello world\"");
    }

    #[test]
    fn test_number() {
        let s = detect_placeholders("The answer is 42 or 3.14");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].text, "42");
        assert_eq!(s[1].text, "3.14");
    }

    #[test]
    fn test_percentage() {
        let s = detect_placeholders("Increase by 15%");
        assert_eq!(s.len(), 1);
        assert!(s[0].text.contains("15"));
        assert!(s[0].text.contains("%"));
    }

    #[test]
    fn test_email() {
        let s = detect_placeholders("Contact user@example.com for help");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].text, "user@example.com");
        assert_eq!(s[0].kind, SlotKind::Entity);
    }

    #[test]
    fn test_url() {
        let s = detect_placeholders("Visit https://example.com today");
        assert!(!s.is_empty());
        assert!(s[0].text.starts_with("https"));
    }

    #[test]
    fn test_iso_date() {
        let s = detect_placeholders("Today is 2026-06-02, hi");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].text, "2026-06-02");
    }

    #[test]
    fn test_cn_date() {
        let s = detect_placeholders("今天是2026年6月2日");
        assert_eq!(s.len(), 1);
        assert!(s[0].text.contains("年"));
        assert!(s[0].text.contains("月"));
    }

    #[test]
    fn test_no_placeholders() {
        let s = detect_placeholders("Just a plain sentence.");
        assert!(s.is_empty());
    }

    #[test]
    fn test_no_brace_pseudo_placeholder() {
        // Unmatched { should not be detected
        let s = detect_placeholders("Use { to mean literal brace");
        assert!(s.is_empty());
    }
}
