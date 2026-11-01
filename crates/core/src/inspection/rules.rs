//! Explicit three-valued property matching. Unknown never means equal.
use super::{invalid, model::*};
use crate::domain::Result;
use unicode_normalization::UnicodeNormalization;

#[derive(Clone)]
pub(crate) struct Properties {
    pub filename: Option<String>,
    pub extension: Option<String>,
    pub bytes: u64,
    pub modified_millis: Option<i64>,
    pub created_millis: Option<i64>,
    pub readonly: Option<bool>,
}

impl PropertyRules {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.name.ignored_characters.chars().count() > 128 || self.tolerance_millis > 86_400_000
        {
            return Err(invalid(
                "Use at most 128 ignored characters and a filesystem-date tolerance of at most one day.",
            ));
        }
        if ![
            self.filename,
            self.extension,
            self.size,
            self.modified,
            self.created,
            self.readonly,
            self.content,
        ]
        .into_iter()
        .any(|selected| selected)
        {
            return Err(invalid(
                "Choose at least one property rule. Unknown values do not silently relax it.",
            ));
        }
        Ok(())
    }
}
pub(crate) fn name_key(name: Option<&str>, policy: &NamePolicy) -> Option<String> {
    let name = name?;
    let name = if policy.unicode_nfc {
        name.nfc().collect::<String>()
    } else {
        name.to_owned()
    };
    let filtered = name
        .chars()
        .filter(|character| {
            !(policy.ignored_characters.contains(*character)
                || policy.ignore_whitespace && character.is_whitespace())
        })
        .collect::<String>();
    let filtered = if policy.lowercase {
        filtered.to_lowercase()
    } else {
        filtered
    };
    let key = if policy.unicode_nfc {
        filtered.nfc().collect()
    } else {
        filtered
    };
    if key.is_empty() { None } else { Some(key) }
}
fn optional<T: PartialEq>(a: Option<T>, b: Option<T>) -> Truth {
    match (a, b) {
        (Some(a), Some(b)) => {
            if a == b {
                Truth::Match
            } else {
                Truth::Different
            }
        }
        _ => Truth::Unknown,
    }
}
fn dates(a: Option<i64>, b: Option<i64>, tolerance: u32) -> Truth {
    match (a, b) {
        (Some(a), Some(b)) => {
            if (i128::from(a) - i128::from(b)).unsigned_abs() <= u128::from(tolerance) {
                Truth::Match
            } else {
                Truth::Different
            }
        }
        _ => Truth::Unknown,
    }
}
pub(crate) fn compare(
    a: &Properties,
    b: &Properties,
    rules: &PropertyRules,
    content: Truth,
) -> Result<PropertyEvidence> {
    rules.validate()?;
    let mut evidence = Vec::with_capacity(7);
    if rules.filename {
        evidence.push(optional(
            name_key(a.filename.as_deref(), &rules.name),
            name_key(b.filename.as_deref(), &rules.name),
        ));
    }
    if rules.extension {
        evidence.push(optional(
            a.extension.as_ref().map(|s| s.to_lowercase()),
            b.extension.as_ref().map(|s| s.to_lowercase()),
        ));
    }
    if rules.size {
        evidence.push(if a.bytes == b.bytes {
            Truth::Match
        } else {
            Truth::Different
        });
    }
    if rules.modified {
        evidence.push(dates(
            a.modified_millis,
            b.modified_millis,
            rules.tolerance_millis,
        ));
    }
    if rules.created {
        evidence.push(dates(
            a.created_millis,
            b.created_millis,
            rules.tolerance_millis,
        ));
    }
    if rules.readonly {
        evidence.push(optional(a.readonly, b.readonly));
    }
    if rules.content {
        evidence.push(content);
    }
    let unknown = evidence
        .iter()
        .filter(|&&value| value == Truth::Unknown)
        .count() as u32;
    let matched = evidence
        .iter()
        .filter(|&&value| value == Truth::Match)
        .count() as u32;
    let selected = evidence.len() as u32;
    let result = match rules.logic {
        RuleLogic::All if evidence.contains(&Truth::Different) => Truth::Different,
        RuleLogic::Any if matched > 0 => Truth::Match,
        _ if unknown > 0 => Truth::Unknown,
        RuleLogic::All => Truth::Match,
        RuleLogic::Any => Truth::Different,
    };
    Ok(PropertyEvidence {
        result,
        unknown_criteria: unknown,
        matched_criteria: matched,
        selected_criteria: selected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rules() -> PropertyRules {
        PropertyRules {
            logic: RuleLogic::All,
            filename: true,
            extension: false,
            size: false,
            modified: false,
            created: true,
            readonly: false,
            content: false,
            tolerance_millis: 0,
            name: NamePolicy::default(),
        }
    }
    fn file(name: &str) -> Properties {
        Properties {
            filename: Some(name.into()),
            extension: Some("JPG".into()),
            bytes: 100,
            modified_millis: Some(-1000),
            created_millis: None,
            readonly: None,
        }
    }
    #[test]
    fn missing_dates_are_unknown_and_all_any_have_explicit_three_valued_semantics() {
        let (a, b) = (file("same"), file("same"));
        let mut policy = rules();
        let all = compare(&a, &b, &policy, Truth::Unknown).unwrap();
        assert_eq!(all.result, Truth::Unknown);
        assert_eq!(all.unknown_criteria, 1);
        policy.logic = RuleLogic::Any;
        assert_eq!(
            compare(&a, &b, &policy, Truth::Unknown).unwrap().result,
            Truth::Match
        );
        let different = file("different");
        assert_eq!(
            compare(&a, &different, &policy, Truth::Unknown)
                .unwrap()
                .result,
            Truth::Unknown
        );
        policy.logic = RuleLogic::All;
        assert_eq!(
            compare(&a, &different, &policy, Truth::Unknown)
                .unwrap()
                .result,
            Truth::Different
        );
    }
    #[test]
    fn normalization_is_explicit_preserves_source_names_and_empty_keys_never_match() {
        let mut policy = NamePolicy {
            unicode_nfc: true,
            lowercase: true,
            ignore_whitespace: true,
            ignored_characters: "_-".into(),
        };
        assert_eq!(
            name_key(Some(" É_c-l a i r "), &policy),
            name_key(Some("E\u{301}CLAIR"), &policy)
        );
        assert_eq!(name_key(Some(" _- \t"), &policy), None);
        policy.unicode_nfc = false;
        assert_ne!(
            name_key(Some("é"), &policy),
            name_key(Some("e\u{301}"), &policy)
        );
        assert_eq!(name_key(None, &policy), None);
    }
    #[test]
    fn dates_use_filesystem_fields_exact_tolerance_and_overflow_safe_differences() {
        assert_eq!(dates(Some(-1001), Some(-1000), 1), Truth::Match);
        assert_eq!(dates(Some(-1002), Some(-1000), 1), Truth::Different);
        assert_eq!(dates(None, None, 0), Truth::Unknown);
        assert_eq!(
            dates(Some(i64::MIN), Some(i64::MAX), 86_400_000),
            Truth::Different
        );
    }
}
