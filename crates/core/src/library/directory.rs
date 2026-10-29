//! Read-only, immediate-child catalog navigation. Native path bytes stay distinct
//! even when two filenames have the same lossy display string. IDs are never FS inputs.
use super::{filters::invalid, model::*, native, store};
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
};
use rusqlite::{OptionalExtension, params, params_from_iter, types::Value};
use serde::{Deserialize, Serialize};
pub(super) fn encode_id(bytes: &[u8]) -> String {
    let mut out = String::from("d");
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
pub(super) fn decode_id(id: &str) -> Result<Vec<u8>> {
    let hex = id
        .strip_prefix('d')
        .ok_or_else(|| invalid("Invalid directory identity."))?;
    if hex.len() > 32768
        || !hex.len().is_multiple_of(2)
        || !hex.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(invalid("Invalid directory identity."));
    }
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text =
                std::str::from_utf8(pair).map_err(|_| invalid("Invalid directory identity."))?;
            u8::from_str_radix(text, 16).map_err(|_| invalid("Invalid directory identity."))
        })
        .collect()
}
type Item = (u8, Vec<u8>, String, String, Option<(AssetSummary, bool)>);
#[derive(Serialize, Deserialize)]
struct Cursor {
    scope: String,
    kind: u8,
    key: String,
    before: bool,
}
fn node(raw: &[u8], name: Option<&str>, state: String) -> Result<CatalogDirectory> {
    let path = native::decode_path(raw)?;
    Ok(CatalogDirectory {
        id: encode_id(raw),
        name: name.map(str::to_owned).unwrap_or_else(|| {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        }),
        state,
    })
}
const CONTENTS: &str = "SELECT 0 AS kind,native_path AS native,'' AS asset_id,CASE WHEN state NOT IN ('blocked','excluded','missing') AND seen_scan<>coalesce((SELECT seen_scan FROM directories r WHERE r.root_id=?1 AND r.native_path=x''),seen_scan) THEN 'retained' ELSE state END AS state FROM directories WHERE root_id=?1 AND parent_native=?2 AND state NOT IN ('excluded','missing') UNION ALL SELECT 1 AS kind,relative_native AS native,id AS asset_id,'' AS state FROM assets WHERE root_id=?1 AND folder_native=?2 AND presence NOT IN ('excluded','missing') AND name LIKE ?3 ESCAPE '\\'";
pub fn browse(
    catalog: &Catalog,
    root_id: String,
    directory_id: Option<String>,
    name: String,
    cursor: Option<String>,
    limit: u32,
) -> Result<DirectoryPage> {
    if uuid::Uuid::parse_str(&root_id).is_err() || name.len() > 256 || !(1..=240).contains(&limit) {
        return Err(invalid("Invalid directory request, filename or page size."));
    }
    let raw = decode_id(directory_id.as_deref().unwrap_or("d"))?;
    let scope = blake3::hash(
        &serde_json::to_vec(&(&root_id, &raw, &name))
            .map_err(|e| invalid("Invalid directory query.").caused_by(e))?,
    )
    .to_hex()
    .to_string();
    let parsed = cursor
        .map(|value| {
            if value.len() > 70000 {
                return Err(invalid("Invalid directory cursor."));
            }
            let value: Cursor =
                serde_json::from_str(&value).map_err(|_| invalid("Invalid directory cursor."))?;
            if value.scope != scope || value.kind > 1 {
                return Err(invalid(
                    "Directory cursor belongs to another folder or search.",
                ));
            }
            decode_id(&value.key)?;
            Ok(value)
        })
        .transpose()?;
    catalog.call(move|db| {
        let root_label:String=db.query_row("SELECT label FROM roots WHERE id=?",[&root_id],|r|r.get(0))?;
        let state:Option<String>=db.query_row("SELECT CASE WHEN d.state NOT IN ('blocked','excluded','missing') AND d.seen_scan<>coalesce((SELECT seen_scan FROM directories r WHERE r.root_id=d.root_id AND r.native_path=x''),d.seen_scan) THEN 'retained' ELSE d.state END FROM directories d WHERE d.root_id=? AND d.native_path=?",params![root_id,raw],|r|r.get(0)).optional()?;
        if !raw.is_empty() && state.is_none() {return Err(invalid("This folder is not in the indexed catalog."));}
        let directory=node(&raw,raw.is_empty().then_some(root_label.as_str()),state.clone().unwrap_or_else(||"incomplete".into()))?;
        if directory.state=="excluded" || directory.state=="blocked" {return Err(AppError::new(ErrorCode::Unsupported,"This catalog entry cannot be browsed."));}
        let path=native::decode_path(&raw)?;
        let mut ancestors=Vec::new();
        for ancestor in path.ancestors().skip(1) {
            if ancestors.len()>=65 {return Err(invalid("Directory exceeds the indexed depth limit."));}
            let bytes=native::encode_path(ancestor);
            ancestors.push(node(&bytes,bytes.is_empty().then_some(root_label.as_str()),"retained".into())?);
        }
        if raw.is_empty() {ancestors.clear();}
        ancestors.reverse();
        let pattern=format!("%{}%",store::escape_like(&name));
        let base=vec![Value::Text(root_id.clone()),Value::Blob(raw.clone()),Value::Text(pattern)];
        // Folders are deliberately kept visible during filename search, so
        // narrowing files cannot hide the navigation needed to reach them.
        let total=db.query_row(&format!("SELECT count(*) FROM ({CONTENTS})"),params_from_iter(base.clone()),|r|r.get(0))?;
        let before=parsed.as_ref().is_some_and(|c|c.before);
        let mut values=base.clone();
        let predicate=if let Some(c)=&parsed {values.push(i64::from(c.kind).into());values.push(decode_id(&c.key)?.into());if before {"WHERE (kind,native)<(?4,?5)"} else {"WHERE (kind,native)>(?4,?5)"}} else {""};
        let order=if before {"DESC"}else{"ASC"};
        let mut items=db.prepare(&format!("SELECT e.kind,e.native,e.asset_id,e.state,a.* FROM ({CONTENTS}) e LEFT JOIN assets a ON a.id=e.asset_id {predicate} ORDER BY e.kind {order},e.native {order} LIMIT {}",limit+1))?.query_map(params_from_iter(values),|r|{let kind=r.get::<_,u8>(0)?;Ok((kind,r.get::<_,Vec<u8>>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,if kind==1{Some((store::asset_row(r)?,r.get::<_,bool>("is_media")?))}else{None}))})?.collect::<rusqlite::Result<Vec<_>>>()?;
        items.truncate(limit as usize);if before {items.reverse();}
        let make_cursor=|item:&Item,before:bool| -> Result<String> {serde_json::to_string(&Cursor{scope:scope.clone(),kind:item.0,key:encode_id(&item.1),before}).map_err(|e|invalid("Could not encode directory cursor.").caused_by(e))};
        let has_edge=|item:&Item,cmp:&str| -> Result<bool> {let mut args=base.clone();args.push(i64::from(item.0).into());args.push(item.1.clone().into());Ok(db.query_row(&format!("SELECT EXISTS(SELECT 1 FROM ({CONTENTS}) WHERE (kind,native){cmp}(?4,?5))"),params_from_iter(args),|r|r.get(0))?)};
        let previous_cursor=items.first().filter(|_|parsed.is_some()).map(|item| -> Result<Option<String>> {Ok(if has_edge(item,"<")? {Some(make_cursor(item,true)?)} else {None})}).transpose()?.flatten();
        let next_cursor=items.last().map(|item| -> Result<Option<String>> {Ok(if has_edge(item,">")? {Some(make_cursor(item,false)?)} else {None})}).transpose()?.flatten();
        let mut entries=Vec::with_capacity(items.len());
        for (kind,key,_id,state,asset) in items {
            if kind==0 {entries.push(DirectoryEntry::Folder{directory:node(&key,None,state)?});}
            else {
                let (asset,media)=asset.ok_or_else(||invalid("The indexed file is no longer available."))?;
                entries.push(DirectoryEntry::File{asset:Box::new(asset),media});
            }
        }
        let revision=db.query_row("SELECT CAST(revision AS TEXT) FROM library_state WHERE singleton=1",[],|r|r.get(0))?;
        Ok(DirectoryPage{directory,ancestors,entries,total,previous_cursor,next_cursor,revision})
    })
}
