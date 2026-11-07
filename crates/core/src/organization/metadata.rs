//! Explicit naming observations. Never infer places, camera classes or tag priority.
use super::{OrganizationIssue, OrganizationTemplate, PreviewSource};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Default, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub country: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub place: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub subcategory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub subject: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub camera_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub brand: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub file_format: Option<String>,
}
impl OrganizationMetadata {
    pub(super) fn bounded(&self) -> bool {
        [
            &self.country,
            &self.region,
            &self.city,
            &self.place,
            &self.tag,
            &self.subcategory,
            &self.subject,
            &self.camera_type,
            &self.brand,
            &self.model,
            &self.file_format,
        ]
        .iter()
        .all(|value| value.as_ref().is_none_or(|s| s.len() <= 512))
    }
}
pub(super) const TOKENS: &[&str] = &[
    "country",
    "region",
    "city",
    "place",
    "tag",
    "subcategory",
    "subject",
    "camera_type",
    "brand",
    "model",
    "camera",
    "file_format",
];
pub(crate) fn requires_metadata(template: &OrganizationTemplate) -> bool {
    TOKENS
        .iter()
        .any(|name| template.pattern.contains(&format!("{{{name}}}")))
}
pub(super) fn value(
    name: &str,
    source: &PreviewSource,
    extension: &str,
) -> std::result::Result<String, OrganizationIssue> {
    use OrganizationIssue::*;
    let empty = OrganizationMetadata::default();
    let metadata = source.metadata.as_ref().unwrap_or(&empty);
    let (value, missing) = match name {
        "country" => (&metadata.country, MissingLocation),
        "region" => (&metadata.region, MissingLocation),
        "city" => (&metadata.city, MissingLocation),
        "place" => (&metadata.place, MissingLocation),
        "tag" => (&metadata.tag, MissingTag),
        "subcategory" => (&metadata.subcategory, MissingTag),
        "subject" => (&metadata.subject, MissingTag),
        "camera_type" => (&metadata.camera_type, MissingCameraMetadata),
        "brand" => (&metadata.brand, MissingCameraMetadata),
        "model" => (&metadata.model, MissingCameraMetadata),
        "camera" => {
            let brand = metadata
                .brand
                .as_deref()
                .filter(|s| !s.is_empty())
                .ok_or(MissingCameraMetadata)?;
            let model = metadata
                .model
                .as_deref()
                .filter(|s| !s.is_empty())
                .ok_or(MissingCameraMetadata)?;
            // Preserve an already branded model; join distinct observed values.
            if model.eq_ignore_ascii_case(brand)
                || model
                    .get(..brand.len())
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(brand))
                    && model
                        .get(brand.len()..)
                        .is_some_and(|tail| tail.starts_with([' ', '-', '_']))
            {
                return Ok(model.into());
            }
            return Ok(format!("{brand}-{model}"));
        }
        "file_format" => {
            let format = metadata
                .file_format
                .as_deref()
                .unwrap_or(extension)
                .to_ascii_uppercase();
            return Ok(match format.as_str() {
                "ARW" | "CR2" | "CR3" | "NEF" | "NRW" | "ORF" | "RAF" | "RW2" | "PEF" | "SRW"
                | "DNG" | "RAW" | "X3F" | "3FR" | "IIQ" | "RWL" | "ERF" | "MOS" => "RAW".into(),
                "JPG" | "JPE" | "JPEG" => "JPEG".into(),
                "TIF" | "TIFF" => "TIFF".into(),
                _ => format,
            });
        }
        _ => return Err(MissingValue),
    };
    value
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .ok_or(missing)
}
