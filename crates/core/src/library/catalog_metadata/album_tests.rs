use super::tests::{fixture, selection};
use super::*;

#[test]
fn albums_create_atomically_reject_names_and_survive_reopen() {
    let (temp, catalog, ids) = fixture();
    let request_create = |name: &str, asset_ids: Vec<String>| {
        request(
            &catalog,
            CatalogMetadataRequest::CreateAlbum {
                name: name.into(),
                asset_ids,
            },
        )
    };
    let CatalogMetadataReply::Collection(album) =
        request_create("  Weekends 港  ", ids[..2].to_vec()).unwrap()
    else {
        panic!()
    };
    assert_eq!(album.name, "Weekends 港");
    assert_eq!(album.member_count, 2);
    assert!(request_create("weekends 港", vec![]).is_err());
    assert!(request_create("family", vec![]).is_err());
    assert!(
        request_create(
            "Invalid",
            vec![ids[0].clone(), uuid::Uuid::new_v4().to_string()]
        )
        .is_err()
    );
    assert!(request_create("Repeated", vec![ids[0].clone(), ids[0].clone()]).is_err());
    assert_eq!(catalog.call(|db| Ok(collections(db)?.len())).unwrap(), 1);
    let CatalogMetadataReply::Membership(repeated) = request(
        &catalog,
        CatalogMetadataRequest::CollectionMembers {
            id: album.id.clone(),
            expected_version: album.version,
            expected_count: 1,
            selection: selection(&ids[..1]),
            remove: false,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(repeated.changed, 0);
    assert_eq!(repeated.collection.member_count, 2);
    let id = album.id.clone();
    drop(catalog);
    let reopened = Catalog::open(temp.path().join("Catalog.stillharbour")).unwrap();
    reopened
        .call(move |db| {
            let saved = collection(db, &id)?;
            assert_eq!(saved.member_count, 2);
            assert_eq!(saved.name, "Weekends 港");
            Ok(())
        })
        .unwrap();
}

#[test]
fn scanned_keywords_group_multiple_albums_respect_removal_and_preserve_observations() {
    let (_temp, catalog, ids) = fixture();
    let first = ids[0].clone();
    catalog
        .call(move |db| {
            let before: (Vec<u8>, String, String) = db.query_row(
                "SELECT relative_native,revision,raw_metadata FROM assets WHERE id=?",
                [&first],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            assert!(albums::import_scanned_keywords(db, &first)?);
            assert!(!albums::import_scanned_keywords(db, &first)?);
            let page = albums::list(db, String::new(), 0)?;
            for name in ["Embedded", "Imported", "Unchanged", "Family"] {
                let album = page.items.iter().find(|a| a.name == name).unwrap();
                assert!(album.from_tags);
                assert_eq!(album.photo_count, 1);
                assert_eq!(album.covers.len(), 1);
                assert!(album.query.include_missing);
                assert_eq!(album.query.tag, name);
            }
            // Catalog removal writes the existing tombstone trigger. A subsequent
            // scan must not resurrect the observed source keyword.
            db.execute(
                "UPDATE assets SET tags='[\"Family\",\"Imported\",\"Unchanged\"]' WHERE id=?",
                [&first],
            )?;
            assert!(!albums::import_scanned_keywords(db, &first)?);
            assert!(albums::list(db, "embedded".into(), 0)?.items.is_empty());
            let after: (Vec<u8>, String, String) = db.query_row(
                "SELECT relative_native,revision,raw_metadata FROM assets WHERE id=?",
                [&first],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            assert_eq!(before, after);
            assert_eq!(
                albums::list(db, "travel".into(), 0)?.items[0].photo_count,
                2
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn malformed_or_overflowing_source_keywords_do_not_partially_import() {
    let (_temp, catalog, ids) = fixture();
    let first = ids[0].clone();
    catalog.call(move |db| {
        for raw in [serde_json::json!([{"Keywords":["Good","\u{0}"]}]), serde_json::json!([{"Keywords": (0..65).map(|i| format!("tag-{i}")).collect::<Vec<_>>() }])] {
            db.execute("UPDATE assets SET raw_metadata=? WHERE id=?", params![raw.to_string(),first])?;
            assert!(!albums::import_scanned_keywords(db, &first)?);
            assert_eq!(db.query_row("SELECT tags FROM assets WHERE id=?", [&first], |r| r.get::<_,String>(0))?, "[\"Family\"]");
            assert!(db.query_row("SELECT metadata_error FROM assets WHERE id=?", [&first], |r| r.get::<_,String>(0))?.contains("automatically"));
        }
        Ok(())
    }).unwrap();
}

#[test]
fn album_search_pages_are_bounded_and_exclude_archived_custom_albums() {
    let (_temp, catalog, _) = fixture();
    catalog
        .call(|db| {
            for n in 0..55 {
                create_collection(
                    db,
                    CollectionKind::Album,
                    format!("Collection {n:02}"),
                    None,
                )?;
            }
            let page = albums::list(db, "collection".into(), 0)?;
            assert_eq!(page.total, 55);
            assert_eq!(page.items.len(), 48);
            assert_eq!(page.next_offset, Some(48));
            let second = albums::list(db, "collection".into(), 48)?;
            assert_eq!(second.items.len(), 7);
            assert!(second.next_offset.is_none());
            db.execute(
                "UPDATE catalog_collections SET archived=1 WHERE id=?",
                [&page.items[0].id],
            )?;
            assert_eq!(albums::list(db, "collection".into(), 0)?.total, 54);
            assert!(albums::check_name(db, CollectionKind::Album, "collection 00", None).is_err());
            Ok(())
        })
        .unwrap();
}

#[test]
fn metadata_publication_imports_keywords_only_for_the_matching_source_revision() {
    let (_temp, catalog, ids) = fixture();
    let metadata = |keyword: &str| store::Metadata {
        raw: serde_json::json!([{"Subject":[keyword]}]).to_string(),
        ..store::Metadata::default()
    };
    store::metadata(
        &catalog,
        ids[0].clone(),
        "stale-revision".into(),
        Some(metadata("Stale")),
        None,
        false,
    )
    .unwrap();
    catalog
        .call(move |db| {
            assert!(albums::list(db, "Stale".into(), 0)?.items.is_empty());
            Ok(())
        })
        .unwrap();
    store::metadata(
        &catalog,
        ids[0].clone(),
        "original-revision".into(),
        Some(metadata("New observation")),
        None,
        false,
    )
    .unwrap();
    catalog
        .call(|db| {
            assert_eq!(
                albums::list(db, "New observation".into(), 0)?.items[0].photo_count,
                1
            );
            Ok(())
        })
        .unwrap();
}
