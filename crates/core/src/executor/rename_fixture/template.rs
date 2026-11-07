//! Existing template policy projected onto the five closed generated members.
//! No source path or existing library can enter this adapter.
use super::*;
use crate::organization::{self, OrganizationTemplate, PreviewSource};

pub(super) fn pairs(template: &OrganizationTemplate) -> Result<Vec<(String, String)>> {
    let sources = [0, 2, 3, 4]
        .into_iter()
        .map(|index| {
            let name = MEMBERS[index].0;
            PreviewSource {
                id: index.to_string(),
                root_id: "generated-template-root".into(),
                root_identity: "constructor-owned".into(),
                native_path: name.as_bytes().to_vec(),
                relative_path: Some(name.into()),
                display_path: name.into(),
                revision: "0".repeat(64),
                bytes: payload(index).len() as u64,
                capture: None,
                metadata: None,
                available: true,
                companion_candidates: u32::from(index == 0),
            }
        })
        .collect();
    let preview = organization::preview("generated-template-v1".into(), template.clone(), sources)?;
    if preview.blocked != 0 {
        return Err(failed(
            "The generated template has blocked targets. Generated files have no capture dates; choose an explicit missing-date policy.",
        ));
    }
    let mut pairs = MEMBERS
        .iter()
        .map(|(name, _)| (name.to_string(), String::new()))
        .collect::<Vec<_>>();
    for row in preview.rows {
        let index: usize = row
            .asset_id
            .parse()
            .map_err(|_| failed("Invalid generated template member."))?;
        let target = row
            .target_relative_path
            .ok_or_else(|| failed("A generated target is missing."))?;
        if target.contains('/') {
            return Err(failed(
                "Generated in-place organization currently accepts filename templates in the same parent. Use source-retaining copy for folder templates.",
            ));
        }
        pairs[index].1 = target;
    }
    pairs[1].1 = crate::library::bundle_confirmation::target(
        &pairs[0].1,
        MEMBERS[1].0,
        &crate::operations::BundleRelation::SameStem,
    )?;
    Ok(pairs)
}

impl Fixture {
    pub(super) fn from_template(template: OrganizationTemplate) -> Result<Self> {
        // Resolve all policy errors before creating any native fixture.
        let pairs = pairs(&template)?;
        let borrowed = pairs
            .iter()
            .map(|(before, after)| (before.as_str(), after.as_str()))
            .collect::<Vec<_>>();
        let mut fixture = Self::generated_members(&borrowed)?;
        if !fixture.plan.issues.is_empty() {
            return Err(failed(
                "Generated template targets conflict under observed native name semantics.",
            ));
        }
        fixture.template = Some(template);
        Ok(fixture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_template_rejects_blocked_nested_and_colliding_policies() {
        for pattern in [
            "{capture:%Y}.{ext}",
            "../{original_stem}.{ext}",
            "folder/{original_stem}.{ext}",
            "same.{ext}",
            "{unknown}",
        ] {
            let template = OrganizationTemplate {
                pattern: pattern.into(),
                event: String::new(),
                missing_date: organization::MissingDatePolicy::Block,
                filename_stem_case: None,
            };
            assert!(pairs(&template).is_err(), "{pattern}");
        }
    }
}
