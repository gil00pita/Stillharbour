//! Bounded namespace-aware XMP merge. Original packets are retained separately.
use crate::domain::{AppError, ErrorCode, Result};
use quick_xml::{
    Writer,
    events::{BytesStart, Event},
    name::ResolveResult,
    reader::NsReader,
};
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const DC: &str = "http://purl.org/dc/elements/1.1/";
const XMP: &str = "http://ns.adobe.com/xap/1.0/";
const XML: &str = "http://www.w3.org/XML/1998/namespace";
fn invalid() -> AppError {
    AppError::new(
        ErrorCode::InvalidInput,
        "This XMP packet is malformed, ambiguous or exceeds the supported safe-merge bounds. Every source remains unchanged.",
    )
}
fn known(uri: &str, local: &str) -> bool {
    (uri == DC && matches!(local, "subject" | "description"))
        || (uri == XMP && local == "Rating")
        || (uri == "urn:stillharbour:metadata:1"
            && matches!(local, "Rating" | "Rejected" | "Favorite" | "AssetId"))
}
fn escape(value: &str) -> Result<String> {
    if value.chars().any(|c| {
        matches!(c, '\u{fffe}' | '\u{ffff}') || (c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    }) {
        return Err(invalid());
    }
    Ok(quick_xml::escape::escape(value).into_owned())
}
pub(super) fn merge(
    source: Option<&[u8]>,
    tags: &[String],
    caption: &str,
    rating: u8,
    rejected: bool,
    favorite: bool,
    id: &str,
) -> Result<Vec<u8>> {
    let empty=br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"></rdf:RDF></x:xmpmeta>"#;
    let source = source.unwrap_or(empty);
    if source.len() > 1024 * 1024 || tags.len() > 64 || rating > 5 {
        return Err(invalid());
    }
    let text = std::str::from_utf8(source).map_err(|_| invalid())?;
    escape(text)?;
    escape(&quick_xml::escape::unescape(text).map_err(|_| invalid())?)?;
    let mut reader = NsReader::from_str(text);
    let mut out = Writer::new(Vec::new());
    let mut depth = 0usize;
    let mut rdf_depth = None;
    let mut description = None;
    let mut skip = None;
    let mut rdf_count = 0;
    let mut injected = false;
    let mut roots = 0;
    for count in 0..50001 {
        if count == 50000 {
            return Err(invalid());
        }
        let (namespace, event) = reader.read_resolved_event().map_err(|_| invalid())?;
        let uri = match namespace {
            ResolveResult::Bound(n) => n.as_ref().to_owned(),
            ResolveResult::Unbound => String::new(),
            ResolveResult::Unknown(_) => return Err(invalid()),
        };
        let event = event.into_owned();
        match &event {
            Event::DocType(_) => return Err(invalid()),
            Event::Decl(decl) => {
                if depth != 0 || roots != 0 {
                    return Err(invalid());
                }
                if decl
                    .encoding()
                    .transpose()
                    .map_err(|_| invalid())?
                    .is_some_and(|e| !e.eq_ignore_ascii_case("utf-8"))
                {
                    return Err(invalid());
                }
            }
            Event::Start(start) | Event::Empty(start) => {
                if depth == 0 {
                    roots += 1;
                    if roots > 1 {
                        return Err(invalid());
                    }
                }
                for (i, attribute) in start.attributes().enumerate() {
                    if i >= 128 {
                        return Err(invalid());
                    }
                    let attribute = attribute.map_err(|_| invalid())?;
                    attribute
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(|_| invalid())?;
                    let (namespace, _) = reader.resolver().resolve_attribute(attribute.key);
                    if matches!(namespace, ResolveResult::Unknown(_)) {
                        return Err(invalid());
                    }
                }
                let local = start.local_name();
                let is_start = matches!(event, Event::Start(_));
                if uri == RDF && local.as_ref() == "RDF" {
                    if rdf_count != 0 || !is_start {
                        return Err(invalid());
                    }
                    rdf_count += 1;
                    rdf_depth = Some(depth);
                }
                if skip.is_none()
                    && description == depth.checked_sub(1)
                    && known(&uri, local.as_ref())
                {
                    for attr in start.attributes() {
                        let attr = attr.map_err(|_| invalid())?;
                        if !attr.key.as_ref().starts_with("xmlns") {
                            return Err(invalid());
                        }
                    }
                    if is_start {
                        skip = Some(depth);
                    }
                    if is_start {
                        depth += 1;
                    }
                    continue;
                }
                if skip.is_some() {
                    if uri != RDF || !matches!(local.as_ref(), "Bag" | "Alt" | "Seq" | "li") {
                        return Err(invalid());
                    }
                    for attr in start.attributes() {
                        let attr = attr.map_err(|_| invalid())?;
                        let (ns, name) = reader.resolver().resolve_attribute(attr.key);
                        let allowed = matches!(ns,ResolveResult::Bound(n) if n.as_ref()==XML)
                            && name.as_ref() == "lang"
                            && attr.value.as_ref() == "x-default";
                        if !allowed {
                            return Err(invalid());
                        }
                    }
                    if is_start {
                        depth += 1;
                    }
                    continue;
                }
                if uri == RDF
                    && local.as_ref() == "Description"
                    && rdf_depth == depth.checked_sub(1)
                {
                    let name = start.name().as_ref().to_owned();
                    let mut rewritten = BytesStart::new(name);
                    for attr in start.attributes() {
                        let attr = attr.map_err(|_| invalid())?;
                        let (ns, name) = reader.resolver().resolve_attribute(attr.key);
                        let remove =
                            matches!(ns,ResolveResult::Bound(n) if known(n.as_ref(),name.as_ref()));
                        if !remove {
                            rewritten.push_attribute(attr);
                        }
                    }
                    if is_start {
                        description = Some(depth);
                        out.write_event(Event::Start(rewritten))
                            .map_err(|_| invalid())?;
                    } else {
                        out.write_event(Event::Empty(rewritten))
                            .map_err(|_| invalid())?;
                    }
                    if is_start {
                        depth += 1;
                    }
                    continue;
                }
                if is_start {
                    depth += 1;
                    if depth > 64 {
                        return Err(invalid());
                    }
                }
            }
            Event::End(end) => {
                depth = depth.checked_sub(1).ok_or_else(invalid)?;
                if let Some(level) = skip {
                    if depth == level {
                        skip = None;
                    }
                    continue;
                }
                if description == Some(depth) {
                    description = None;
                }
                if uri == RDF && end.local_name().as_ref() == "RDF" && rdf_depth == Some(depth) {
                    let mut packet = format!(
                        "<r:Description xmlns:r=\"{}\" xmlns:dc=\"{}\" xmlns:xmp=\"{}\" xmlns:sh=\"urn:stillharbour:metadata:1\" r:about=\"\" xmp:Rating=\"{}\" sh:Rating=\"{}\" sh:Rejected=\"{}\" sh:Favorite=\"{}\" sh:AssetId=\"{}\"><dc:subject><r:Bag>",
                        RDF,
                        DC,
                        XMP,
                        if rejected { -1 } else { i32::from(rating) },
                        rating,
                        rejected,
                        favorite,
                        escape(id)?
                    );
                    for tag in tags {
                        packet.push_str(&format!("<r:li>{}</r:li>", escape(tag)?));
                    }
                    packet.push_str(&format!("</r:Bag></dc:subject><dc:description><r:Alt><r:li xml:lang=\"x-default\">{}</r:li></r:Alt></dc:description></r:Description>",escape(caption)?));
                    out.get_mut().extend_from_slice(packet.as_bytes());
                    injected = true;
                }
            }
            Event::Text(value) if depth == 0 && !value.as_ref().trim().is_empty() => {
                return Err(invalid());
            }
            Event::Eof => {
                if depth != 0 || skip.is_some() || rdf_count != 1 || !injected {
                    return Err(invalid());
                }
                let bytes = out.into_inner();
                if bytes.len() > 2 * 1024 * 1024 {
                    return Err(invalid());
                }
                return Ok(bytes);
            }
            _ => {
                if skip.is_some() {
                    continue;
                }
            }
        }
        out.write_event(event).map_err(|_| invalid())?;
    }
    Err(invalid())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_unknown_namespace_editing_instructions_and_replaces_known_aliases() {
        let input=br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><r:RDF xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><r:Description xmlns:d="http://purl.org/dc/elements/1.1/" xmlns:q="urn:third-party" q:Recipe="keep &amp; retain"><q:History><q:Step>untouched</q:Step></q:History><d:subject><r:Bag><r:li>old</r:li></r:Bag></d:subject></r:Description></r:RDF></x:xmpmeta>"#;
        let output = merge(
            Some(input),
            &["港 & <label>".into()],
            "Caption <safe>",
            4,
            true,
            false,
            "asset",
        )
        .unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("q:Recipe=\"keep &amp; retain\""));
        assert!(text.contains("<q:Step>untouched</q:Step>"));
        assert!(!text.contains(">old<"));
        assert!(text.contains("港 &amp; &lt;label&gt;"));
        assert!(text.contains("xmp:Rating=\"-1\""));
        assert!(input.windows(5).any(|v| v == b">old<"));
    }
    #[test]
    fn rejects_dtd_unknown_qualifiers_and_malformed_packets() {
        for raw in [
            "<!DOCTYPE x [<!ENTITY e SYSTEM 'file:///etc/passwd'>]><x/>",
            "<x>",
            "<r:RDF xmlns:r='http://www.w3.org/1999/02/22-rdf-syntax-ns#'><r:Description xmlns:d='http://purl.org/dc/elements/1.1/'><d:subject><private xmlns='urn:unknown'/></d:subject></r:Description></r:RDF>",
        ] {
            assert!(merge(Some(raw.as_bytes()), &[], "", 0, false, false, "id").is_err());
        }
    }
}
