//! Bounded source keyword observation for the Windows metadata worker.
//! XML entities/DTDs never gain filesystem or network access.
use super::*;
use quick_xml::{events::Event, name::ResolveResult, reader::NsReader};
use std::collections::BTreeSet;

fn add(tags: &mut BTreeSet<String>, text: &str) -> Result<()> {
    let text = text.trim();
    if text.is_empty() || text.len() > 128 || text.chars().any(char::is_control) {
        return Err(invalid());
    }
    tags.insert(text.into());
    if tags.len() > 64 {
        return Err(invalid());
    }
    Ok(())
}
fn xmp(bytes: &[u8], tags: &mut BTreeSet<String>) -> Result<()> {
    if bytes.len() > 1024 * 1024 {
        return Err(invalid());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    let mut reader = NsReader::from_str(text);
    let mut depth = 0usize;
    let mut subject = None;
    let mut item = None;
    let mut value = String::new();
    for _ in 0..50000 {
        let (namespace, event) = reader.read_resolved_event().map_err(|_| invalid())?;
        let uri = match namespace {
            ResolveResult::Bound(n) => n.as_ref().to_owned(),
            ResolveResult::Unbound => String::new(),
            ResolveResult::Unknown(_) => return Err(invalid()),
        };
        match event {
            Event::DocType(_) => return Err(invalid()),
            Event::GeneralRef(reference) => {
                let reference = reference.as_ref();
                let encoded = format!("&{reference};");
                let decoded = quick_xml::escape::unescape(&encoded).map_err(|_| invalid())?;
                if item.is_some() {
                    value.push_str(&decoded);
                    if value.len() > 128 {
                        return Err(invalid());
                    }
                }
            }
            Event::Start(start) => {
                depth += 1;
                if depth > 32 {
                    return Err(invalid());
                }
                if uri == "http://purl.org/dc/elements/1.1/"
                    && start.local_name().as_ref() == "subject"
                {
                    subject = Some(depth);
                } else if subject.is_some()
                    && uri == "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                    && start.local_name().as_ref() == "li"
                {
                    item = Some(depth);
                    value.clear();
                }
            }
            Event::Text(text) if item.is_some() => {
                value.push_str(&text.xml_content(quick_xml::XmlVersion::Implicit1_0));
                if value.len() > 128 {
                    return Err(invalid());
                }
            }
            Event::CData(text) if item.is_some() => {
                value.push_str(&text.xml_content(quick_xml::XmlVersion::Implicit1_0));
                if value.len() > 128 {
                    return Err(invalid());
                }
            }
            Event::End(_) => {
                if item == Some(depth) {
                    add(tags, &value)?;
                    item = None;
                }
                if subject == Some(depth) {
                    subject = None;
                }
                depth = depth.checked_sub(1).ok_or_else(invalid)?;
            }
            Event::Eof => return if depth == 0 { Ok(()) } else { Err(invalid()) },
            _ => {}
        }
    }
    Err(invalid())
}
fn iptc(bytes: &[u8], tags: &mut BTreeSet<String>) -> Result<()> {
    if bytes.len() > 1024 * 1024 {
        return Err(invalid());
    }
    if bytes.starts_with(b"8BIM") {
        let mut position = 0;
        while position < bytes.len() {
            let header = bytes.get(position..position + 7).ok_or_else(invalid)?;
            if &header[..4] != b"8BIM" {
                return Err(invalid());
            }
            let id = u16::from_be_bytes([header[4], header[5]]);
            let name_length = usize::from(header[6]) + 1;
            let size_at = position + 6 + name_length + name_length % 2;
            let size = bytes.get(size_at..size_at + 4).ok_or_else(invalid)?;
            let length = u32::from_be_bytes(size.try_into().map_err(|_| invalid())?) as usize;
            let data = bytes
                .get(size_at + 4..size_at + 4 + length)
                .ok_or_else(invalid)?;
            if id == 0x0404 {
                iptc(data, tags)?;
            }
            position = size_at + 4 + length + length % 2;
        }
        return Ok(());
    }
    let mut position = 0;
    let mut utf8 = false;
    while position < bytes.len() {
        let header = bytes.get(position..position + 5).ok_or_else(invalid)?;
        if header[0] != 0x1c || header[3] & 0x80 != 0 {
            return Err(invalid());
        }
        let length = usize::from(u16::from_be_bytes([header[3], header[4]]));
        let data = bytes
            .get(position + 5..position + 5 + length)
            .ok_or_else(invalid)?;
        match (header[1], header[2]) {
            (1, 90) => {
                if data != b"\x1b%G" {
                    return Err(invalid());
                }
                utf8 = true;
            }
            (2, 25) => {
                if !utf8 && !data.is_ascii() {
                    return Err(invalid());
                }
                add(tags, std::str::from_utf8(data).map_err(|_| invalid())?)?;
            }
            _ => {}
        }
        position += 5 + length;
    }
    Ok(())
}
pub(super) fn extract(bytes: &[u8], exif: Option<&exif::Exif>) -> Result<Vec<String>> {
    let mut tags = BTreeSet::new();
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|_| invalid())?;
    if let Some(packet) = decoder.xmp_metadata().map_err(|_| invalid())? {
        xmp(&packet, &mut tags)?;
    }
    if let Some(packet) = decoder.iptc_metadata().map_err(|_| invalid())? {
        iptc(&packet, &mut tags)?;
    }
    if let Some(field) =
        exif.and_then(|e| e.get_field(Tag(exif::Context::Tiff, 0x9c9e), In::PRIMARY))
    {
        let Value::Byte(bytes) = &field.value else {
            return Err(invalid());
        };
        if bytes.len() > 16384 || bytes.len() % 2 != 0 {
            return Err(invalid());
        }
        let units = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect::<Vec<_>>();
        let text = String::from_utf16(&units).map_err(|_| invalid())?;
        for tag in text
            .trim_end_matches('\0')
            .split(';')
            .filter(|s| !s.trim().is_empty())
        {
            add(&mut tags, tag)?;
        }
    }
    Ok(tags.into_iter().collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn namespace_tags_unicode_and_untrusted_xml_are_bounded_data() {
        let xml = br#"<a:subject xmlns:a="http://purl.org/dc/elements/1.1/" xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><r:Bag><r:li>Coast &amp; Water</r:li><r:li>Coast &amp; Water</r:li><r:li><![CDATA[<script>data</script>]]></r:li></r:Bag></a:subject>"#;
        let mut tags = BTreeSet::new();
        xmp(xml, &mut tags).unwrap();
        assert_eq!(tags.len(), 2);
        assert!(
            xmp(
                b"<!DOCTYPE a SYSTEM 'file:///private'><a/>",
                &mut BTreeSet::new()
            )
            .is_err()
        );
        assert!(xmp(b"<a>", &mut BTreeSet::new()).is_err());
        assert!(xmp(&vec![b'x'; 1024 * 1024 + 1], &mut BTreeSet::new()).is_err());
    }
    #[test]
    fn jpeg_iptc_resources_and_utf16_xpkeywords_use_existing_decoders() {
        use image::ImageEncoder;
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
            .write_image(&[50u8; 8 * 8 * 3], 8, 8, image::ExtendedColorType::Rgb8)
            .unwrap();
        let data = b"\x1c\x02\x19\0\x05Coast";
        let mut resource = b"8BIM\x04\x04\0\0".to_vec();
        resource.extend((data.len() as u32).to_be_bytes());
        resource.extend(data);
        let mut packet = b"Photoshop 3.0\0".to_vec();
        packet.extend(resource);
        let mut tagged = vec![0xff, 0xd8, 0xff, 0xed];
        tagged.extend(((packet.len() + 2) as u16).to_be_bytes());
        tagged.extend(packet);
        tagged.extend(&jpeg[2..]);
        assert_eq!(extract(&tagged, None).unwrap(), vec!["Coast"]);
        let words = "Coast;港\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let mut tiff = b"II\x2a\0\x08\0\0\0".to_vec();
        tiff.extend(1u16.to_le_bytes());
        tiff.extend(0x9c9eu16.to_le_bytes());
        tiff.extend(1u16.to_le_bytes());
        tiff.extend((words.len() as u32).to_le_bytes());
        tiff.extend(26u32.to_le_bytes());
        tiff.extend(0u32.to_le_bytes());
        tiff.extend(words);
        let exif = exif::Reader::new().read_raw(tiff).unwrap();
        assert_eq!(extract(&jpeg, Some(&exif)).unwrap(), vec!["Coast", "港"]);
    }
    #[test]
    fn iptc_repeated_keywords_invalid_lengths_and_encoding() {
        let mut tags = BTreeSet::new();
        iptc(b"\x1c\x02\x19\0\x05Coast\x1c\x02\x19\0\x05Coast", &mut tags).unwrap();
        assert_eq!(tags.len(), 1);
        assert!(iptc(b"\x1c\x02\x19\0\x05abc", &mut tags).is_err());
        assert!(iptc(b"\x1c\x02\x19\0\x01\xff", &mut tags).is_err());
    }
}
