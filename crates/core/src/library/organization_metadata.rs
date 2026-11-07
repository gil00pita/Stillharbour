//! Bounded projections of existing catalog observations for naming only.
use super::catalog_metadata::model::TagCategory;
use crate::organization::OrganizationMetadata;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn project(
    raw: Option<&str>,
    tags: &str,
    format: Option<String>,
    categories: &[TagCategory],
) -> OrganizationMetadata {
    let raw = raw
        .filter(|s| s.len() <= 1024 * 1024)
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
    let object = raw
        .as_ref()
        .and_then(|v| v.as_array())
        .filter(|v| v.len() == 1)
        .and_then(|v| v.first())
        .and_then(|v| v.as_object());
    let observed = |keys: &[&str]| {
        let values = keys
            .iter()
            .filter_map(|key| object?.get(*key)?.as_str())
            .filter(|s| !s.trim().is_empty())
            .collect::<BTreeSet<_>>();
        if values.len() == 1 {
            values
                .first()
                .filter(|s| s.len() <= 512)
                .map(|s| (*s).to_owned())
        } else {
            None
        }
    };
    let mut metadata = OrganizationMetadata {
        country: observed(&["Country", "Country-PrimaryLocationName", "CountryName"]),
        region: observed(&["State", "Province-State", "ProvinceState"]),
        city: observed(&["City"]),
        place: observed(&["Sub-location", "Sublocation", "Location"]),
        camera_type: observed(&["CameraType"]),
        brand: observed(&["Make"]),
        model: observed(&["Model"]),
        file_format: format,
        ..Default::default()
    };
    if tags.len() > 16384 {
        return metadata;
    }
    let Some(tags) = serde_json::from_str::<Vec<String>>(tags)
        .ok()
        .filter(|v| v.len() <= 64 && v.iter().all(|s| s.len() <= 128))
    else {
        return metadata;
    };
    let by_label = categories
        .iter()
        .map(|c| (c.label.as_str(), c))
        .collect::<BTreeMap<_, _>>();
    let by_id = categories
        .iter()
        .map(|c| (c.id.as_str(), c))
        .collect::<BTreeMap<_, _>>();
    let mut paths = Vec::with_capacity(tags.len());
    for tag in &tags {
        let mut path = Vec::new();
        if let Some(node) = by_label.get(tag.as_str()) {
            let mut cursor = Some(*node);
            let mut visited = BTreeSet::new();
            while let Some(node) = cursor {
                if !visited.insert(&node.id) || path.len() >= 16 {
                    return metadata;
                }
                path.push(node.label.clone());
                cursor = node
                    .parent_id
                    .as_ref()
                    .and_then(|id| by_id.get(id.as_str()).copied());
            }
            path.reverse();
        } else {
            path.push(tag.clone());
        }
        if !path.is_empty() {
            paths.push(path);
        }
    }
    // Assigned ancestor tags do not make an otherwise unique leaf ambiguous.
    let leaves = paths
        .iter()
        .filter(|path| {
            !paths
                .iter()
                .any(|other| other.len() > path.len() && other.starts_with(path))
        })
        .collect::<Vec<_>>();
    let level = |index: usize| {
        if leaves.is_empty() || leaves.iter().any(|path| path.len() <= index) {
            return None;
        }
        let prefixes = leaves
            .iter()
            .map(|path| &path[..=index])
            .collect::<BTreeSet<_>>();
        if prefixes.len() != 1 {
            return None;
        }
        leaves[0].get(index).cloned()
    };
    metadata.tag = level(0);
    metadata.subcategory = level(1);
    metadata.subject = level(2);
    metadata
}

#[cfg(test)]
mod tests {
    use super::*;
    fn category(id: &str, name: &str, parent: Option<&str>) -> TagCategory {
        TagCategory {
            id: id.into(),
            label: name.into(),
            parent_id: parent.map(str::to_owned),
            version: 1,
        }
    }
    #[test]
    fn projects_only_explicit_bounded_names_and_keeps_ambiguous_observations_missing() {
        let raw = r#"[{"Country":"Portugal","City":"Lisbon","State":"Lisbon","Sub-location":"Belem","Make":"Sony","Model":"A7-IV","GPSLatitude":38.7,"GPSLongitude":-9.1}]"#;
        let fields = project(Some(raw), "[]", Some("ARW".into()), &[]);
        assert_eq!(fields.country.as_deref(), Some("Portugal"));
        assert_eq!(fields.city.as_deref(), Some("Lisbon"));
        assert_eq!(fields.place.as_deref(), Some("Belem"));
        assert!(
            fields.camera_type.is_none(),
            "Make/model do not establish camera type"
        );
        let fields = project(
            Some(
                r#"[{"Country":"Portugal","CountryName":"Spain","City":42,"GPSLatitude":38.7,"GPSLongitude":-9.1}]"#,
            ),
            "[]",
            None,
            &[],
        );
        assert!(fields.country.is_none());
        assert!(fields.city.is_none());
        assert!(project(Some("not JSON"), "[]", None, &[]).country.is_none());
        assert!(
            project(Some(&"x".repeat(1024 * 1024 + 1)), "[]", None, &[])
                .country
                .is_none()
        );
    }
    #[test]
    fn hierarchy_uses_catalog_parents_and_never_arbitrary_tag_order() {
        let categories = [
            category("a", "Nature", None),
            category("b", "Birds", Some("a")),
            category("c", "Kingfishers", Some("b")),
            category("d", "Mammals", Some("a")),
        ];
        let result = project(
            None,
            r#"["Nature","Kingfishers","Birds"]"#,
            None,
            &categories,
        );
        assert_eq!(result.tag.as_deref(), Some("Nature"));
        assert_eq!(result.subcategory.as_deref(), Some("Birds"));
        assert_eq!(result.subject.as_deref(), Some("Kingfishers"));
        let branches = project(None, r#"["Birds","Mammals"]"#, None, &categories);
        assert_eq!(branches.tag.as_deref(), Some("Nature"));
        assert!(branches.subcategory.is_none());
        let unrelated = project(None, r#"["Nature","Architecture"]"#, None, &categories);
        assert!(unrelated.tag.is_none());
        assert!(project(None, "[]", None, &categories).tag.is_none());
    }
}
