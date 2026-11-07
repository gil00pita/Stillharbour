//! Explicit illustrative inputs: no catalog/source observations or mutation IDs.
use super::*;

pub fn template_examples(template: OrganizationTemplate) -> Result<OrganizationExamplePreview> {
    validate_template(&template)?;
    let sources = (0..4)
        .map(|index| {
            let path = format!("Unsorted/IMG_{:04}.JPG", 421 + index);
            PreviewSource {
                id: format!("illustrative-{index}"),
                root_id: "illustrative-library".into(),
                root_identity: "illustrative-only".into(),
                native_path: Vec::new(),
                display_path: path.clone(),
                relative_path: Some(path),
                revision: "illustrative-only".into(),
                bytes: 0,
                capture: (index != 3).then(|| {
                    if index == 2 {
                        "2026-10-15T09:15:00"
                    } else {
                        "2026-11-07T14:30:52"
                    }
                    .into()
                }),
                available: true,
                companion_candidates: 0,
                metadata: Some(OrganizationMetadata {
                    country: Some("Portugal".into()),
                    region: Some("Algarve".into()),
                    city: Some("Lagos".into()),
                    place: Some("Old-Town".into()),
                    tag: Some("Nature".into()),
                    subcategory: Some("Birds".into()),
                    subject: Some("Kingfishers".into()),
                    camera_type: Some("Mirrorless".into()),
                    brand: Some("Sony".into()),
                    model: Some("A7-IV".into()),
                    file_format: Some("JPEG".into()),
                }),
            }
        })
        .collect::<Vec<_>>();
    let mut preview = preview(
        "illustrative-only".into(),
        template.clone(),
        sources.clone(),
    )?;
    for (row, source) in preview.rows.iter_mut().zip(&sources) {
        row.capture_missing = Some(capture_missing(source));
    }
    Ok(OrganizationExamplePreview {
        sample_only: true,
        rows: preview.rows,
    })
}
