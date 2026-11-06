//! Read-only EXIF parsing inside the bounded, supervised stdin worker.
use super::*;
mod keywords;
use exif::{In, Tag, Value};
use serde_json::{Map, Value as Json};

fn invalid() -> AppError {
    AppError::new(
        ErrorCode::InvalidInput,
        "The bounded EXIF metadata is malformed or unsupported.",
    )
}
fn text(exif: &exif::Exif, tag: Tag) -> Result<Option<String>> {
    let Some(field) = exif.get_field(tag, In::PRIMARY) else {
        return Ok(None);
    };
    let Value::Ascii(values) = &field.value else {
        return Err(invalid());
    };
    if values.len() != 1 || values[0].len() > 4096 {
        return Err(invalid());
    }
    let bytes = values[0].strip_suffix(&[0]).unwrap_or(&values[0]);
    Ok(Some(
        std::str::from_utf8(bytes)
            .map_err(|_| invalid())?
            .to_owned(),
    ))
}
fn coordinate(
    exif: &exif::Exif,
    tag: Tag,
    reference: Tag,
    positive: &str,
    negative: &str,
) -> Result<Option<f64>> {
    let Some(field) = exif.get_field(tag, In::PRIMARY) else {
        return Ok(None);
    };
    let Some(reference) = text(exif, reference)? else {
        return Ok(None);
    };
    let sign = if reference == positive {
        1.0
    } else if reference == negative {
        -1.0
    } else {
        return Err(invalid());
    };
    let Value::Rational(parts) = &field.value else {
        return Err(invalid());
    };
    if parts.len() != 3 || parts.iter().any(|p| p.denom == 0) {
        return Err(invalid());
    }
    let numbers: Vec<f64> = parts
        .iter()
        .map(|p| f64::from(p.num) / f64::from(p.denom))
        .collect();
    if numbers[1] >= 60.0 || numbers[2] >= 60.0 {
        return Err(invalid());
    }
    Ok(Some(
        sign * (numbers[0] + numbers[1] / 60.0 + numbers[2] / 3600.0),
    ))
}
fn extract(bytes: &[u8]) -> Result<Json> {
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(invalid());
    }
    let report = decode_bytes(bytes, 32, PreviewFormat::Auto)?;
    // Catalog projection applies the observed orientation to source geometry;
    // preview-normalized display dimensions are a separate representation.
    let (width, height) = (report.source_width, report.source_height);
    let mut value = Map::new();
    value.insert("ImageWidth".into(), width.into());
    value.insert("ImageHeight".into(), height.into());
    value.insert(
        "StillharbourMetadataPolicy".into(),
        "windows-exif-v3".into(),
    );
    let exif = match exif::Reader::new().read_from_container(&mut Cursor::new(bytes)) {
        Ok(exif) => Some(exif),
        Err(exif::Error::NotFound(_)) => None,
        Err(_) => return Err(invalid()),
    };
    let keywords = keywords::extract(bytes, exif.as_ref())?;
    if !keywords.is_empty() {
        value.insert(
            "Keywords".into(),
            serde_json::to_value(keywords).map_err(|_| invalid())?,
        );
    }
    if let Some(exif) = exif {
        for (key, tag) in [
            ("Make", Tag::Make),
            ("Model", Tag::Model),
            ("LensModel", Tag::LensModel),
            ("DateTimeOriginal", Tag::DateTimeOriginal),
            ("CreateDate", Tag::DateTimeDigitized),
            ("OffsetTimeOriginal", Tag::OffsetTimeOriginal),
        ] {
            if let Some(text) = text(&exif, tag)? {
                value.insert(key.into(), text.into());
            }
        }
        for (key, tag, reference, positive, negative) in [
            (
                "GPSLatitude",
                Tag::GPSLatitude,
                Tag::GPSLatitudeRef,
                "N",
                "S",
            ),
            (
                "GPSLongitude",
                Tag::GPSLongitude,
                Tag::GPSLongitudeRef,
                "E",
                "W",
            ),
        ] {
            if let Some(number) = coordinate(&exif, tag, reference, positive, negative)? {
                value.insert(key.into(), Json::from(number));
            }
        }
        if let Some(orientation) = exif
            .get_field(Tag::Orientation, In::PRIMARY)
            .and_then(|f| f.value.get_uint(0))
        {
            value.insert("Orientation".into(), orientation.into());
        }
    }
    Ok(Json::Array(vec![Json::Object(value)]))
}

pub(super) fn worker() -> Result<()> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .lock()
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    let value = extract(&bytes)?;
    serde_json::to_writer(std::io::stdout().lock(), &value).map_err(|_| invalid())?;
    Ok(())
}

#[cfg(all(test, feature = "fixture-probes"))]
mod tests {
    use super::*;
    #[test]
    fn tagged_generated_jpegs_expose_multiple_keywords_without_source_changes() {
        let fixture = crate::library_fixture::LibraryFixture::generate_tagged().unwrap();
        let count = fixture.verify_unchanged().unwrap();
        let raw = extract(&std::fs::read(fixture.path().join("Coast/Generated-00.jpg")).unwrap())
            .unwrap();
        assert_eq!(raw[0]["Keywords"], serde_json::json!(["Coast", "Outdoors"]));
        assert_eq!(fixture.verify_unchanged().unwrap(), count);
    }
    #[test]
    fn source_geometry_and_capture_offset_are_observations_not_double_rotation() {
        let fixture = crate::library_fixture::LibraryFixture::generate().unwrap();
        let original_count = fixture.verify_unchanged().unwrap();
        let raw = extract(&std::fs::read(fixture.path().join("Coast/Generated-06.jpg")).unwrap())
            .unwrap();
        assert_eq!(raw[0]["ImageWidth"], 600);
        assert_eq!(raw[0]["ImageHeight"], 450);
        assert_eq!(raw[0]["Orientation"], 6);
        let raw = extract(&std::fs::read(fixture.path().join("Coast/Generated-00.jpg")).unwrap())
            .unwrap();
        assert_eq!(raw[0]["DateTimeOriginal"], "1964:02:29 13:15:20");
        assert_eq!(raw[0]["OffsetTimeOriginal"], "+05:30");
        assert_eq!(fixture.verify_unchanged().unwrap(), original_count);
    }
    #[test]
    fn generated_exif_gps_preserves_zero_reference_signs_and_originals() {
        let fixture = crate::library_fixture::LibraryFixture::generate_geographic().unwrap();
        for (name, latitude, longitude) in [
            ("Zero.jpg", 0.0, 0.0),
            ("West.jpg", 5.0, -179.75),
            ("South.jpg", -33.5, -70.7),
        ] {
            let bytes = std::fs::read(fixture.path().join(name)).unwrap();
            let raw = extract(&bytes).unwrap();
            assert!((raw[0]["GPSLatitude"].as_f64().unwrap() - latitude).abs() < 0.00001);
            assert!((raw[0]["GPSLongitude"].as_f64().unwrap() - longitude).abs() < 0.00001);
            assert_eq!(raw[0]["ImageWidth"], 360);
        }
        let missing = extract(&std::fs::read(fixture.path().join("Unknown.jpg")).unwrap()).unwrap();
        assert!(missing[0].get("GPSLatitude").is_none());
        let invalid = extract(&std::fs::read(fixture.path().join("Invalid.jpg")).unwrap()).unwrap();
        assert_eq!(invalid[0]["GPSLatitude"], 91.0);
        assert_eq!(fixture.verify_unchanged().unwrap(), 9);
    }
    #[test]
    fn malformed_metadata_and_input_overflow_are_explicit_failures() {
        assert!(extract(b"not a photograph").is_err());
        assert!(extract(&vec![0; MAX_INPUT_BYTES as usize + 1]).is_err());
    }
    #[test]
    fn invalid_gps_reference_and_zero_denominator_never_create_coordinates() {
        let fixture = crate::library_fixture::LibraryFixture::generate_geographic().unwrap();
        let bytes = std::fs::read(fixture.path().join("South.jpg")).unwrap();
        let original = exif::Reader::new()
            .read_from_container(&mut Cursor::new(&bytes))
            .unwrap();
        let mut raw = original.buf().to_vec();
        assert_eq!(raw[36], b'S');
        raw[36] = b'X';
        let parsed = exif::Reader::new().read_raw(raw).unwrap();
        assert!(coordinate(&parsed, Tag::GPSLatitude, Tag::GPSLatitudeRef, "N", "S").is_err());
        let mut raw = original.buf().to_vec();
        assert_eq!(&raw[84..88], &1u32.to_le_bytes());
        raw[84..88].fill(0);
        let parsed = exif::Reader::new().read_raw(raw).unwrap();
        assert!(coordinate(&parsed, Tag::GPSLatitude, Tag::GPSLatitudeRef, "N", "S").is_err());
        assert_eq!(fixture.verify_unchanged().unwrap(), 9);
    }
}
