use super::super::model::ScanFilter;
use super::*;

pub(super) fn fixture() -> (tempfile::TempDir, Catalog, Vec<String>) {
    let temp = tempfile::tempdir().unwrap();
    let catalog = Catalog::open(temp.path().join("Catalog.stillharbour")).unwrap();
    let ids = (0..3)
        .map(|_| uuid::Uuid::new_v4().to_string())
        .collect::<Vec<_>>();
    let copy = ids.clone();
    catalog.call(move|db|{
        let root=uuid::Uuid::new_v4().to_string();
        db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter) VALUES(?,'Generated',x'01','Generated','referenced',1,'fixture','fixture','0','0','offline',?)",params![root,serde_json::to_string(&ScanFilter::default()).unwrap()])?;
        for (i,id) in copy.iter().enumerate() {
            db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan,tags,raw_metadata) VALUES(?1,?2,CAST(?1 AS BLOB),?1,x'','',?1,?1,?3,?1,1,'original-revision','2026-11-02','2026-11-02','capture','ready','unavailable','present',1,'fixture',?4,?5)",params![id,root,if i==2{"mp4"}else{"jpg"},if i==0{r#"["Family"]"#}else{r#"["Travel"]"#},r#"[{"Keywords":["Embedded","Unchanged"],"Subject":"Imported"}]"#])?;
        }Ok(())
    }).unwrap();
    (temp, catalog, ids)
}
fn album(catalog: &Catalog, name: &str) -> CatalogCollection {
    match request(
        catalog,
        CatalogMetadataRequest::CreateCollection {
            collection_kind: CollectionKind::Album,
            name: name.into(),
            query: None,
        },
    )
    .unwrap()
    {
        CatalogMetadataReply::Collection(value) => value,
        _ => panic!("collection response"),
    }
}
fn read_notes(catalog: &Catalog, id: &str) -> AssetNotes {
    match request(
        catalog,
        CatalogMetadataRequest::Notes {
            asset_id: id.into(),
        },
    )
    .unwrap()
    {
        CatalogMetadataReply::Notes(value) => value,
        _ => panic!("notes response"),
    }
}
pub(super) fn selection(ids: &[String]) -> AssetSelection {
    AssetSelection::Ids { ids: ids.to_vec() }
}
fn category(catalog: &Catalog, label: &str, parent: Option<String>) -> TagCategory {
    match request(
        catalog,
        CatalogMetadataRequest::SaveTagCategory {
            id: None,
            expected_version: None,
            label: label.into(),
            parent_id: parent,
        },
    )
    .unwrap()
    {
        CatalogMetadataReply::TagCategory(value) => value,
        _ => panic!("category response"),
    }
}

#[test]
fn caption_rejection_and_observed_keywords_survive_reopen_and_never_modify_file_observations() {
    let (temp, catalog, ids) = fixture();
    let notes = read_notes(&catalog, &ids[0]);
    assert_eq!(notes.version, 0);
    assert!(!notes.rejected);
    assert_eq!(
        notes.observed_keywords,
        vec!["Embedded", "Imported", "Unchanged"]
    );
    let before = catalog
        .call(|db| {
            Ok(db.query_row(
                "SELECT relative_native,revision,raw_metadata,tags FROM assets ORDER BY id LIMIT 1",
                [],
                |r| {
                    Ok((
                        r.get::<_, Vec<u8>>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                },
            )?)
        })
        .unwrap();
    request(
        &catalog,
        CatalogMetadataRequest::SaveNotes {
            asset_id: ids[0].clone(),
            expected_version: 0,
            caption: "<script>data only</script>\n港".into(),
            rejected: true,
        },
    )
    .unwrap();
    assert_eq!(read_notes(&catalog, &ids[0]).version, 1);
    assert!(
        request(
            &catalog,
            CatalogMetadataRequest::SaveNotes {
                asset_id: ids[0].clone(),
                expected_version: 0,
                caption: "stale draft".into(),
                rejected: false
            }
        )
        .is_err()
    );
    let after = catalog
        .call(|db| {
            Ok(db.query_row(
                "SELECT relative_native,revision,raw_metadata,tags FROM assets ORDER BY id LIMIT 1",
                [],
                |r| {
                    Ok((
                        r.get::<_, Vec<u8>>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                },
            )?)
        })
        .unwrap();
    assert_eq!(before, after);
    catalog
        .call(|db| {
            assert!(db.execute("DELETE FROM asset_note_history", []).is_err());
            Ok(())
        })
        .unwrap();
    let id = catalog.summary().unwrap().id;
    drop(catalog);
    let reopened = Catalog::open(temp.path().join("Catalog.stillharbour")).unwrap();
    assert_eq!(reopened.summary().unwrap().id, id);
    assert_eq!(
        read_notes(&reopened, &ids[0]).caption,
        "<script>data only</script>\n港"
    );
    assert!(read_notes(&reopened, &ids[0]).rejected);
    let result = store::query(
        &reopened,
        AssetQuery {
            rejected: Some(true),
            ..AssetQuery::default()
        },
        None,
        80,
    )
    .unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.assets[0].id, ids[0]);
    assert_eq!(
        store::query(
            &reopened,
            AssetQuery {
                rejected: Some(false),
                ..AssetQuery::default()
            },
            None,
            80
        )
        .unwrap()
        .total,
        2
    );
}

#[test]
fn note_limits_stale_versions_and_unknown_assets_fail_without_partial_writes() {
    let (_temp, catalog, ids) = fixture();
    for (id, caption) in [
        (ids[0].clone(), "x".repeat(4097)),
        (ids[0].clone(), "null\0byte".into()),
        (uuid::Uuid::new_v4().to_string(), "unknown".into()),
    ] {
        assert!(
            request(
                &catalog,
                CatalogMetadataRequest::SaveNotes {
                    asset_id: id,
                    expected_version: 0,
                    caption,
                    rejected: true
                }
            )
            .is_err()
        );
    }
    assert_eq!(read_notes(&catalog, &ids[0]).version, 0);
    assert!(serde_json::from_str::<CatalogMetadataRequest>(r#"{"kind":"save_notes","asset_id":"x","expected_version":0,"caption":"x","rejected":false,"shell":"do not execute"}"#).is_err());
}

#[test]
fn virtual_album_membership_is_atomic_idempotent_reversible_and_works_offline_for_video() {
    let (_temp, catalog, ids) = fixture();
    let a = album(&catalog, "Album / 港");
    let changed = match request(
        &catalog,
        CatalogMetadataRequest::CollectionMembers {
            id: a.id.clone(),
            expected_version: a.version,
            selection: selection(&ids),
            expected_count: ids.len() as u32,
            remove: false,
        },
    )
    .unwrap()
    {
        CatalogMetadataReply::Membership(value) => value,
        _ => panic!("membership"),
    };
    assert_eq!(changed.changed, 3);
    assert_eq!(changed.collection.member_count, 3);
    let same = match request(
        &catalog,
        CatalogMetadataRequest::CollectionMembers {
            id: a.id.clone(),
            expected_version: changed.collection.version,
            selection: selection(&ids),
            expected_count: ids.len() as u32,
            remove: false,
        },
    )
    .unwrap()
    {
        CatalogMetadataReply::Membership(value) => value,
        _ => panic!("membership"),
    };
    assert_eq!(same.changed, 0);
    assert_eq!(same.collection.version, changed.collection.version);
    let page = store::query(
        &catalog,
        AssetQuery {
            album_id: Some(a.id.clone()),
            ..AssetQuery::default()
        },
        None,
        1,
    )
    .unwrap();
    assert_eq!(page.total, 3);
    assert!(page.next_cursor.is_some());
    let other = album(&catalog, "Other");
    let mut invalid = ids.clone();
    invalid.push(uuid::Uuid::new_v4().to_string());
    assert!(
        request(
            &catalog,
            CatalogMetadataRequest::CollectionMembers {
                id: other.id.clone(),
                expected_version: 1,
                selection: selection(&invalid),
                expected_count: invalid.len() as u32,
                remove: false
            }
        )
        .is_err()
    );
    assert_eq!(
        catalog
            .call(move |db| Ok(collection(db, &other.id)?.member_count))
            .unwrap(),
        0
    );
    let removed = match request(
        &catalog,
        CatalogMetadataRequest::CollectionMembers {
            id: a.id.clone(),
            expected_version: same.collection.version,
            selection: selection(&ids[2..]),
            expected_count: 1,
            remove: true,
        },
    )
    .unwrap()
    {
        CatalogMetadataReply::Membership(value) => value,
        _ => panic!("membership"),
    };
    assert_eq!(removed.changed, 1);
    let archived = match request(
        &catalog,
        CatalogMetadataRequest::ArchiveCollection {
            id: a.id.clone(),
            expected_version: removed.collection.version,
            archived: true,
        },
    )
    .unwrap()
    {
        CatalogMetadataReply::Collection(value) => value,
        _ => panic!("collection"),
    };
    assert_eq!(
        store::query(
            &catalog,
            AssetQuery {
                album_id: Some(a.id.clone()),
                ..AssetQuery::default()
            },
            None,
            80
        )
        .unwrap()
        .total,
        0
    );
    request(
        &catalog,
        CatalogMetadataRequest::ArchiveCollection {
            id: a.id.clone(),
            expected_version: archived.version,
            archived: false,
        },
    )
    .unwrap();
    assert_eq!(
        store::query(
            &catalog,
            AssetQuery {
                album_id: Some(a.id),
                ..AssetQuery::default()
            },
            None,
            80
        )
        .unwrap()
        .total,
        2
    );
}

#[test]
fn saved_searches_store_validated_filters_without_sql_or_permission_authority() {
    let (_temp, catalog, _ids) = fixture();
    let query = AssetQuery {
        extension: "mp4".into(),
        min_rating: Some(0),
        ..AssetQuery::default()
    };
    let saved = match request(
        &catalog,
        CatalogMetadataRequest::CreateCollection {
            collection_kind: CollectionKind::Search,
            name: "Videos".into(),
            query: Some(query.clone()),
        },
    )
    .unwrap()
    {
        CatalogMetadataReply::Collection(value) => value,
        _ => panic!("collection"),
    };
    assert_eq!(saved.query.unwrap(), query);
    assert!(
        request(
            &catalog,
            CatalogMetadataRequest::CreateCollection {
                collection_kind: CollectionKind::Search,
                name: "Unsafe sort".into(),
                query: Some(AssetQuery {
                    sort: "drop table assets".into(),
                    ..AssetQuery::default()
                })
            }
        )
        .is_err()
    );
    assert!(
        request(
            &catalog,
            CatalogMetadataRequest::CreateCollection {
                collection_kind: CollectionKind::Album,
                name: "Cannot carry search".into(),
                query: Some(AssetQuery::default())
            }
        )
        .is_err()
    );
    assert!(serde_json::from_str::<AssetQuery>(r#"{"script":"do not execute"}"#).is_err());
}

#[test]
fn tag_categories_keep_literal_labels_and_reject_cycles_depth_and_stale_reparenting() {
    let (_temp, catalog, ids) = fixture();
    let parent = category(&catalog, "People", None);
    let child = category(&catalog, "Family", Some(parent.id.clone()));
    let direct = store::query(
        &catalog,
        AssetQuery {
            tag: "People".into(),
            ..AssetQuery::default()
        },
        None,
        80,
    )
    .unwrap();
    assert_eq!(direct.total, 0);
    let descendant = store::query(
        &catalog,
        AssetQuery {
            tag: "People".into(),
            tag_descendants: Some(true),
            ..AssetQuery::default()
        },
        None,
        80,
    )
    .unwrap();
    assert_eq!(descendant.total, 1);
    assert_eq!(descendant.assets[0].id, ids[0]);
    assert_eq!(descendant.assets[0].tags, vec!["Family"]);
    assert!(
        request(
            &catalog,
            CatalogMetadataRequest::SaveTagCategory {
                id: Some(parent.id.clone()),
                expected_version: Some(1),
                label: "People".into(),
                parent_id: Some(child.id)
            }
        )
        .is_err()
    );
    assert!(
        request(
            &catalog,
            CatalogMetadataRequest::SaveTagCategory {
                id: Some(parent.id),
                expected_version: Some(0),
                label: "People".into(),
                parent_id: None
            }
        )
        .is_err()
    );
    category(&catalog, "family", None);
    category(&catalog, "A/B", None); // distinct labels, slash is literal
    let mut previous = None;
    for i in 0..16 {
        previous = Some(category(&catalog, &format!("Depth {i}"), previous).id);
    }
    assert!(
        request(
            &catalog,
            CatalogMetadataRequest::SaveTagCategory {
                id: None,
                expected_version: None,
                label: "Too deep".into(),
                parent_id: previous
            }
        )
        .is_err()
    );
}

#[test]
fn consistent_catalog_snapshot_keeps_notes_album_and_tag_relationships_independently_openable() {
    let (temp, catalog, ids) = fixture();
    let a = album(&catalog, "Retained");
    request(
        &catalog,
        CatalogMetadataRequest::CollectionMembers {
            id: a.id,
            expected_version: 1,
            selection: selection(&ids),
            expected_count: ids.len() as u32,
            remove: false,
        },
    )
    .unwrap();
    request(
        &catalog,
        CatalogMetadataRequest::SaveNotes {
            asset_id: ids[0].clone(),
            expected_version: 0,
            caption: "Retained caption".into(),
            rejected: true,
        },
    )
    .unwrap();
    category(&catalog, "Parent", None);
    let output = temp.path().join("Snapshot.stillharbour");
    catalog.snapshot(output.clone()).unwrap();
    let copy = Catalog::open(output).unwrap();
    assert_eq!(read_notes(&copy, &ids[0]).caption, "Retained caption");
    assert!(
        matches!(request(&copy,CatalogMetadataRequest::Collections{}).unwrap(),CatalogMetadataReply::Collections(values) if values[0].member_count==3)
    );
    assert!(
        matches!(request(&copy,CatalogMetadataRequest::TagCategories{}).unwrap(),CatalogMetadataReply::TagCategories(values) if values.len()==1)
    );
}

#[test]
fn frozen_album_selection_keeps_exact_membership_across_collection_creation_and_refuses_shrinkage()
{
    let (_temp, catalog, ids) = fixture();
    let frozen = store::freeze(&catalog, AssetQuery::default()).unwrap();
    let a = album(&catalog, "Created after selection");
    request(
        &catalog,
        CatalogMetadataRequest::CollectionMembers {
            id: a.id,
            expected_version: 1,
            expected_count: frozen.count,
            selection: AssetSelection::Token { id: frozen.id },
            remove: false,
        },
    )
    .unwrap();
    let frozen = store::freeze(&catalog, AssetQuery::default()).unwrap();
    let b = album(&catalog, "No partial memberships");
    let removed = ids[0].clone();
    catalog
        .call(move |db| {
            db.execute("DELETE FROM selection_members WHERE asset_id=?", [&removed])?;
            db.execute("DELETE FROM assets WHERE id=?", [removed])?;
            Ok(())
        })
        .unwrap();
    assert!(
        request(
            &catalog,
            CatalogMetadataRequest::CollectionMembers {
                id: b.id.clone(),
                expected_version: 1,
                expected_count: frozen.count,
                selection: AssetSelection::Token { id: frozen.id },
                remove: false
            }
        )
        .is_err()
    );
    assert_eq!(
        catalog
            .call(move |db| Ok(collection(db, &b.id)?.member_count))
            .unwrap(),
        0
    );
}

#[test]
fn catalog_tag_removal_and_source_rescan_never_resurrect_observed_keywords() {
    let (_temp, catalog, ids) = fixture();
    store::annotate(
        &catalog,
        selection(&ids[..1]),
        super::super::model::AnnotationPatch {
            favorite: None,
            rating: None,
            tags: Some(vec![]),
        },
    )
    .unwrap();
    store::metadata(
        &catalog,
        ids[0].clone(),
        "original-revision".into(),
        Some(store::Metadata {
            raw: r#"[{"Keywords":["Family","New observation"]}]"#.into(),
            ..store::Metadata::default()
        }),
        None,
        false,
    )
    .unwrap();
    let page = store::query(
        &catalog,
        AssetQuery {
            tag: "Family".into(),
            ..AssetQuery::default()
        },
        None,
        80,
    )
    .unwrap();
    assert_eq!(page.total, 0);
    assert_eq!(
        read_notes(&catalog, &ids[0]).observed_keywords,
        vec!["Family", "New observation"]
    );
}

#[test]
fn imported_search_scripts_and_invalid_hierarchy_are_data_errors_without_execution() {
    let (_temp, catalog, _ids) = fixture();
    let search = match request(
        &catalog,
        CatalogMetadataRequest::CreateCollection {
            collection_kind: CollectionKind::Search,
            name: "Search".into(),
            query: Some(AssetQuery::default()),
        },
    )
    .unwrap()
    {
        CatalogMetadataReply::Collection(value) => value,
        _ => panic!("search"),
    };
    catalog
        .call(move |db| {
            db.execute(
                "UPDATE catalog_collections SET query_json=? WHERE id=?",
                params![
                    r#"{"script":"DELETE FROM assets","schedule":true}"#,
                    search.id
                ],
            )?;
            Ok(())
        })
        .unwrap();
    assert!(request(&catalog, CatalogMetadataRequest::Collections {}).is_err());
    let a = category(&catalog, "A", None);
    let b = category(&catalog, "B", Some(a.id.clone()));
    catalog
        .call(move |db| {
            db.execute(
                "UPDATE catalog_tag_categories SET parent_id=? WHERE id=?",
                params![b.id, a.id],
            )?;
            Ok(())
        })
        .unwrap();
    assert!(request(&catalog, CatalogMetadataRequest::TagCategories {}).is_err());
    assert_eq!(
        catalog
            .call(|db| Ok(db.query_row("SELECT count(*) FROM assets", [], |r| r.get::<_, u32>(0))?))
            .unwrap(),
        3
    );
}

#[test]
fn annotation_history_failure_rolls_back_note_version_and_catalog_revision() {
    let (_temp, catalog, ids) = fixture();
    let before = store::revision(&catalog).unwrap();
    catalog.call(|db|{db.execute_batch("CREATE TRIGGER fixture_deny_note_history BEFORE INSERT ON asset_note_history BEGIN SELECT RAISE(ABORT,'generated history denial'); END;")?;Ok(())}).unwrap();
    assert!(
        request(
            &catalog,
            CatalogMetadataRequest::SaveNotes {
                asset_id: ids[0].clone(),
                expected_version: 0,
                caption: "Uncommitted".into(),
                rejected: true
            }
        )
        .is_err()
    );
    let notes = read_notes(&catalog, &ids[0]);
    assert_eq!(notes.version, 0);
    assert!(notes.caption.is_empty());
    assert!(!notes.rejected);
    assert_eq!(store::revision(&catalog).unwrap(), before);
}

#[test]
fn album_review_refuses_expired_frozen_repeated_and_stale_selections_atomically() {
    let (_temp, catalog, ids) = fixture();
    let frozen = store::freeze(&catalog, AssetQuery::default()).unwrap();
    let token = frozen.id.clone();
    catalog
        .call(move |db| {
            db.execute(
                "UPDATE selections SET created_at='2000-01-01T00:00:00Z' WHERE id=?",
                [token],
            )?;
            Ok(())
        })
        .unwrap();
    let a = album(&catalog, "Expired review");
    let revision = store::revision(&catalog).unwrap();
    for (selection, expected_version, expected_count) in [
        (
            AssetSelection::Token { id: frozen.id },
            a.version,
            frozen.count,
        ),
        (selection(&[ids[0].clone(), ids[0].clone()]), a.version, 2),
        (selection(&ids), 0, ids.len() as u32),
    ] {
        assert!(
            request(
                &catalog,
                CatalogMetadataRequest::CollectionMembers {
                    id: a.id.clone(),
                    expected_version,
                    expected_count,
                    selection,
                    remove: false,
                }
            )
            .is_err()
        );
    }
    assert_eq!(store::revision(&catalog).unwrap(), revision);
    catalog
        .call(move |db| {
            let current = collection(db, &a.id)?;
            assert_eq!(current.member_count, 0);
            assert_eq!(current.version, 1);
            Ok(())
        })
        .unwrap();
}

#[test]
fn reviewed_tag_merge_undo_preserves_categories_case_labels_and_rejects_stale_or_replayed_changes()
{
    let (_temp, catalog, ids) = fixture();
    let parent = category(&catalog, "Family", None);
    let child = category(&catalog, "Child", Some(parent.id.clone()));
    let preview = catalog
        .call(|db| tags::preview(db, "Family".into(), Some("Travel".into())))
        .unwrap();
    assert!(preview.merges);
    assert_eq!(preview.affected_count, 1);
    let review = preview.clone();
    catalog
        .call(move |db| tags::apply(db, review.id, review.digest))
        .unwrap();
    let graph = catalog.call(|db| categories(db)).unwrap();
    assert!(
        graph
            .iter()
            .any(|n| n.id == parent.id && n.label == "Travel")
    );
    assert!(
        graph
            .iter()
            .any(|n| n.id == child.id && n.parent_id.as_ref() == Some(&parent.id))
    );
    let revision = store::revision(&catalog).unwrap();
    let id = preview.id.clone();
    catalog
        .call(move |db| tags::undo(db, id, revision))
        .unwrap();
    assert_eq!(
        store::asset(&catalog, ids[0].clone()).unwrap().tags,
        vec!["Family"]
    );
    let restored = catalog.call(|db| categories(db)).unwrap();
    assert!(restored.iter().find(|n| n.id == parent.id).unwrap().version > parent.version);
    assert!(
        catalog
            .call(move |db| tags::apply(db, preview.id, preview.digest))
            .is_err()
    );
    let stale = catalog
        .call(|db| tags::preview(db, "Family".into(), Some("family".into())))
        .unwrap();
    store::annotate(
        &catalog,
        selection(&ids[..1]),
        super::super::model::AnnotationPatch {
            favorite: Some(true),
            rating: None,
            tags: None,
        },
    )
    .unwrap();
    assert!(
        catalog
            .call(move |db| tags::apply(db, stale.id, stale.digest))
            .is_err()
    );
    assert_eq!(
        store::asset(&catalog, ids[0].clone()).unwrap().tags,
        vec!["Family"]
    );
}

#[test]
fn explicit_observed_import_respects_removal_tombstones_across_rescans_and_reopen() {
    let (temp, catalog, ids) = fixture();
    let revision = store::revision(&catalog).unwrap();
    let id = ids[0].clone();
    catalog
        .call(move |db| tags::import_keywords(db, id, revision, false))
        .unwrap();
    let current = store::asset(&catalog, ids[0].clone()).unwrap();
    assert!(current.tags.contains(&"Embedded".into()));
    store::annotate(
        &catalog,
        selection(&ids[..1]),
        super::super::model::AnnotationPatch {
            favorite: None,
            rating: None,
            tags: Some(vec!["Family".into()]),
        },
    )
    .unwrap();
    let revision = store::revision(&catalog).unwrap();
    let id = ids[0].clone();
    catalog
        .call(move |db| tags::import_keywords(db, id, revision, false))
        .unwrap();
    assert_eq!(
        store::asset(&catalog, ids[0].clone()).unwrap().tags,
        vec!["Family"]
    );
    drop(catalog);
    let catalog = Catalog::open(temp.path().join("Catalog.stillharbour")).unwrap();
    let revision = store::revision(&catalog).unwrap();
    let id = ids[0].clone();
    catalog
        .call(move |db| tags::import_keywords(db, id, revision, true))
        .unwrap();
    assert!(
        store::asset(&catalog, ids[0].clone())
            .unwrap()
            .tags
            .contains(&"Embedded".into())
    );
}

#[test]
fn category_merge_cycle_and_history_failure_never_partially_change_tags() {
    let (_temp, catalog, ids) = fixture();
    let parent = category(&catalog, "Family", None);
    category(&catalog, "Descendant", Some(parent.id));
    assert!(
        catalog
            .call(|db| tags::preview(db, "Family".into(), Some("Descendant".into())))
            .is_err()
    );
    let review = catalog
        .call(|db| tags::preview(db, "Family".into(), None))
        .unwrap();
    catalog.call(|db|{db.execute_batch("CREATE TRIGGER fixture_tag_history_denied BEFORE INSERT ON catalog_tag_history BEGIN SELECT RAISE(ABORT,'generated denial'); END;")?;Ok(())}).unwrap();
    assert!(
        catalog
            .call(move |db| tags::apply(db, review.id, review.digest))
            .is_err()
    );
    assert_eq!(
        store::asset(&catalog, ids[0].clone()).unwrap().tags,
        vec!["Family"]
    );
}

#[test]
fn category_only_rename_and_undo_remain_visible_in_history() {
    let (_temp, catalog, _ids) = fixture();
    let original = category(&catalog, "Only category", None);
    let preview = catalog
        .call(|db| tags::preview(db, "Only category".into(), Some("Renamed category".into())))
        .unwrap();
    assert_eq!(preview.affected_count, 0);
    let applied = catalog
        .call(move |db| tags::apply(db, preview.id, preview.digest))
        .unwrap();
    let id = applied.id.clone();
    assert!(
        catalog
            .call(|db| tags::history(db))
            .unwrap()
            .iter()
            .any(|entry| entry.id == id && entry.affected_count == 0)
    );
    let revision = store::revision(&catalog).unwrap();
    let undo = catalog
        .call(move |db| tags::undo(db, id, revision))
        .unwrap();
    let id = undo.id.clone();
    assert!(
        catalog
            .call(|db| tags::history(db))
            .unwrap()
            .iter()
            .any(|entry| entry.id == id && entry.affected_count == 0)
    );
    let restored = catalog.call(|db| categories(db)).unwrap();
    assert!(
        restored
            .iter()
            .any(|node| node.label == original.label && node.version > original.version)
    );
    let revision = store::revision(&catalog).unwrap();
    catalog
        .call(move |db| tags::undo(db, id, revision))
        .unwrap();
    assert!(
        catalog
            .call(|db| categories(db))
            .unwrap()
            .iter()
            .any(|node| node.label == "Renamed category")
    );
}

#[test]
fn tag_undo_compares_category_identity_when_rename_changes_display_order() {
    let (_temp, catalog, ids) = fixture();
    let original = category(&catalog, "Family", None);
    category(&catalog, "Middle", None);
    category(&catalog, "Child", Some(original.id.clone()));
    let preview = catalog
        .call(|db| tags::preview(db, "Family".into(), Some("Zulu".into())))
        .unwrap();
    let applied = catalog
        .call(move |db| tags::apply(db, preview.id, preview.digest))
        .unwrap();
    let revision = store::revision(&catalog).unwrap();
    let undo = catalog
        .call(move |db| tags::undo(db, applied.id, revision))
        .unwrap();
    assert_eq!(
        store::asset(&catalog, ids[0].clone()).unwrap().tags,
        vec!["Family"]
    );
    let revision = store::revision(&catalog).unwrap();
    catalog
        .call(move |db| tags::undo(db, undo.id, revision))
        .unwrap();
    assert_eq!(
        store::asset(&catalog, ids[0].clone()).unwrap().tags,
        vec!["Zulu"]
    );
}

#[test]
fn oversized_imported_tag_history_cannot_partially_restore_annotations() {
    let (_temp, catalog, ids) = fixture();
    let asset = ids[0].clone();
    catalog.call(move |db| {
        let before = serde_json::to_string(&vec!["a".repeat(65537)]).unwrap();
        db.execute("INSERT INTO catalog_tag_history(operation_id,asset_id,before_tags,after_tags,created_at) VALUES('oversized-history',?,?,?,?)", params![asset,before,"[\"Family\"]",store::now()])?;
        Ok(())
    }).unwrap();
    let revision = store::revision(&catalog).unwrap();
    let expected = revision.clone();
    assert!(
        catalog
            .call(move |db| tags::undo(db, "oversized-history".into(), expected))
            .is_err()
    );
    assert_eq!(store::revision(&catalog).unwrap(), revision);
    assert_eq!(
        store::asset(&catalog, ids[0].clone()).unwrap().tags,
        vec!["Family"]
    );
}
