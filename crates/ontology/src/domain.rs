//! Domain auto-detection: identify which ontology domain a prompt belongs to.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A domain profile with characteristic keywords.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainProfile {
    /// Domain identifier (e.g., "finance", "legal", "medical")
    pub id: String,
    /// Display name
    pub name: String,
    /// Language this profile applies to
    pub lang: String,
    /// Characteristic keywords with weights
    pub keywords: Vec<(String, f64)>,
}

/// Result of domain detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainMatch {
    /// Matched domain ID
    pub domain: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Keywords that matched
    pub matched_keywords: Vec<String>,
}

/// Detect the most likely domain for a given text.
///
/// Simple keyword-overlap scoring: count weighted keyword hits,
/// normalize by total keyword weight.
pub fn detect_domain(text: &str, profiles: &[DomainProfile]) -> Option<DomainMatch> {
    let mut best: Option<DomainMatch> = None;

    for profile in profiles {
        let mut score = 0.0;
        let mut total_weight = 0.0;
        let mut matched = Vec::new();

        for (keyword, weight) in &profile.keywords {
            total_weight += weight;
            if text.contains(keyword.as_str()) {
                score += weight;
                matched.push(keyword.clone());
            }
        }

        if total_weight > 0.0 {
            let confidence = score / total_weight;
            if confidence > 0.05 {
                let dm = DomainMatch {
                    domain: profile.id.clone(),
                    confidence,
                    matched_keywords: matched,
                };
                match &best {
                    Some(b) if b.confidence >= dm.confidence => {}
                    _ => best = Some(dm),
                }
            }
        }
    }

    best
}

/// Build default domain profiles for Chinese.
pub fn default_zh_profiles() -> Vec<DomainProfile> {
    vec![
        DomainProfile {
            id: "finance".into(),
            name: "金融/经济".into(),
            lang: "zh".into(),
            keywords: vec![
                ("财政".into(), 1.0),
                ("税收".into(), 0.9),
                ("货币".into(), 0.9),
                ("宏观".into(), 0.8),
                ("微观".into(), 0.8),
                ("均衡".into(), 0.7),
                ("GDP".into(), 0.8),
                ("通胀".into(), 0.8),
                ("利率".into(), 0.8),
                ("汇率".into(), 0.7),
                ("央行".into(), 0.8),
                ("市场".into(), 0.5),
                ("供给".into(), 0.7),
                ("需求".into(), 0.7),
                ("弹性".into(), 0.6),
                ("边际".into(), 0.6),
                ("效用".into(), 0.6),
                ("成本".into(), 0.5),
                ("收益".into(), 0.5),
                ("投资".into(), 0.6),
                ("消费".into(), 0.5),
                ("贸易".into(), 0.6),
                ("预算".into(), 0.7),
                ("赤字".into(), 0.7),
                ("公债".into(), 0.8),
                ("转移支付".into(), 0.8),
                ("政府购买".into(), 0.8),
                ("乘数".into(), 0.7),
                ("IS-LM".into(), 0.9),
                ("AD-AS".into(), 0.9),
            ],
        },
        DomainProfile {
            id: "legal".into(),
            name: "法律".into(),
            lang: "zh".into(),
            keywords: vec![
                ("法律".into(), 1.0),
                ("法规".into(), 0.9),
                ("宪法".into(), 0.9),
                ("民法".into(), 0.9),
                ("刑法".into(), 0.9),
                ("合同".into(), 0.7),
                ("诉讼".into(), 0.8),
                ("判决".into(), 0.8),
                ("司法".into(), 0.8),
                ("行政".into(), 0.6),
                ("权利".into(), 0.5),
                ("义务".into(), 0.5),
                ("侵权".into(), 0.8),
                ("仲裁".into(), 0.8),
            ],
        },
        DomainProfile {
            id: "medical".into(),
            name: "医学".into(),
            lang: "zh".into(),
            keywords: vec![
                ("临床".into(), 1.0),
                ("诊断".into(), 0.9),
                ("治疗".into(), 0.8),
                ("病理".into(), 0.9),
                ("药理".into(), 0.9),
                ("症状".into(), 0.7),
                ("患者".into(), 0.6),
                ("手术".into(), 0.7),
                ("药物".into(), 0.6),
                ("感染".into(), 0.7),
                ("免疫".into(), 0.7),
                ("肿瘤".into(), 0.8),
            ],
        },
        DomainProfile {
            id: "tech".into(),
            name: "技术/编程".into(),
            lang: "zh".into(),
            keywords: vec![
                ("算法".into(), 1.0),
                ("数据结构".into(), 0.9),
                ("编程".into(), 0.8),
                ("接口".into(), 0.6),
                ("架构".into(), 0.7),
                ("数据库".into(), 0.8),
                ("网络".into(), 0.5),
                ("操作系统".into(), 0.8),
                ("编译".into(), 0.8),
                ("并发".into(), 0.7),
                ("分布式".into(), 0.7),
                ("机器学习".into(), 0.8),
                ("深度学习".into(), 0.8),
                ("神经网络".into(), 0.8),
            ],
        },
    ]
}

/// Build default domain profiles for English.
pub fn default_en_profiles() -> Vec<DomainProfile> {
    vec![
        DomainProfile {
            id: "finance".into(),
            name: "Finance".into(),
            lang: "en".into(),
            keywords: vec![
                ("fiscal".into(), 1.0),
                ("monetary".into(), 0.9),
                ("inflation".into(), 0.8),
                ("GDP".into(), 0.8),
                ("equilibrium".into(), 0.7),
                ("aggregate".into(), 0.6),
                ("supply".into(), 0.5),
                ("demand".into(), 0.5),
                ("interest rate".into(), 0.8),
                ("exchange rate".into(), 0.7),
                ("central bank".into(), 0.8),
                ("taxation".into(), 0.9),
                ("budget deficit".into(), 0.8),
                ("multiplier".into(), 0.7),
            ],
        },
        DomainProfile {
            id: "legal".into(),
            name: "Legal".into(),
            lang: "en".into(),
            keywords: vec![
                ("statute".into(), 1.0),
                ("jurisdiction".into(), 0.9),
                ("plaintiff".into(), 0.9),
                ("defendant".into(), 0.9),
                ("court".into(), 0.7),
                ("contract".into(), 0.6),
                ("liability".into(), 0.8),
                ("tort".into(), 0.9),
                ("arbitration".into(), 0.8),
                ("amendment".into(), 0.7),
            ],
        },
        DomainProfile {
            id: "tech".into(),
            name: "Technology".into(),
            lang: "en".into(),
            keywords: vec![
                ("algorithm".into(), 1.0),
                ("data structure".into(), 0.9),
                ("compiler".into(), 0.8),
                ("distributed".into(), 0.7),
                ("concurrency".into(), 0.8),
                ("API".into(), 0.6),
                ("database".into(), 0.7),
                ("architecture".into(), 0.7),
                ("machine learning".into(), 0.8),
                ("neural network".into(), 0.8),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_finance_zh() {
        let profiles = default_zh_profiles();
        let text = "请阐述财政政策调整对宏观经济均衡状态的影响机制";
        let result = detect_domain(text, &profiles);
        assert!(result.is_some());
        let dm = result.unwrap();
        assert_eq!(dm.domain, "finance");
        assert!(dm.confidence > 0.1);
        assert!(dm.matched_keywords.contains(&"财政".to_string()));
    }

    #[test]
    fn test_detect_tech_zh() {
        let profiles = default_zh_profiles();
        let text = "解释分布式系统中的一致性算法原理";
        let result = detect_domain(text, &profiles);
        assert!(result.is_some());
        assert_eq!(result.unwrap().domain, "tech");
    }

    #[test]
    fn test_detect_no_match() {
        let profiles = default_zh_profiles();
        let text = "今天天气真好";
        let result = detect_domain(text, &profiles);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_finance_en() {
        let profiles = default_en_profiles();
        let text = "Explain the fiscal policy impact on GDP equilibrium";
        let result = detect_domain(text, &profiles);
        assert!(result.is_some());
        assert_eq!(result.unwrap().domain, "finance");
    }
}
