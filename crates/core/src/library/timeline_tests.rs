use super::*;

fn fixture() -> (tempfile::TempDir, Catalog) {
    let owned = tempfile::tempdir().unwrap();
    let catalog = Catalog::open(owned.path().join("Timeline.stillharbour")).unwrap();
    catalog.call(|db| {
        db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter,file_count) VALUES('timeline','Generated',x'01','Generated','referenced',1,'fixture','fixture','fixture','fixture','offline','{}',900)", [])?;
        let tx = db.transaction()?;
        for index in 0..900 {
            let id = uuid::Uuid::from_u128(index + 1).to_string();
            let date = match index { 0..400 => "2026-07-01T12:00:00", 400..700 => "2025-02-01T12:00:00", _ => "" };
            tx.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan,favorite,tags) VALUES(?1,'timeline',CAST(?1 AS BLOB),?1,x'','',?1,?1,'jpg',?1,1,'fixture','2026-11-05T00:00:00Z',?2,'fixture','ready','unavailable',?3,1,'fixture',?4,'[]')", params![id, date, if index % 7 == 0 {"missing"} else {"present"}, index % 3 == 0])?;
        }
        tx.commit()?;
        Ok(())
    }).unwrap();
    (owned, catalog)
}

#[test]
fn ordinal_seek_matches_filtered_stable_order_including_ties_and_undated() {
    let (_owned, catalog) = fixture();
    for sort in ["capture", "added"] {
        for favorites_only in [true, false] {
            let query = AssetQuery {
                sort: sort.into(),
                favorites_only,
                include_missing: true,
                ..Default::default()
            };
            let mut assets = Vec::new();
            let mut cursor = None;
            loop {
                let page = super::query(&catalog, query.clone(), cursor, 240).unwrap();
                assert_eq!(page.start_index, Some(assets.len() as u32));
                assets.extend(page.assets);
                cursor = page.next_cursor;
                if cursor.is_none() {
                    break;
                }
            }
            for index in [0, 1, 239, 240, 510, 899, u32::MAX] {
                let expected = index.min(assets.len() as u32 - 1);
                let page = seek_index(&catalog, query.clone(), index, 240).unwrap();
                assert_eq!(page.start_index, Some(expected));
                assert_eq!(page.assets[0].id, assets[expected as usize].id);
                assert!(page.assets.len() <= 240);
                assert_eq!(
                    page.assets
                        .iter()
                        .map(|asset| &asset.id)
                        .collect::<Vec<_>>(),
                    assets[expected as usize..]
                        .iter()
                        .take(240)
                        .map(|asset| &asset.id)
                        .collect::<Vec<_>>()
                );
                let packed: PackedAssetPage = page.into();
                assert_eq!(packed.start_index, Some(expected));
            }
        }
    }
}

#[test]
fn ordinal_seek_rejects_invalid_scope_and_handles_no_matching_photos() {
    let (_owned, catalog) = fixture();
    for limit in [0, 241, u32::MAX] {
        assert!(seek_index(&catalog, AssetQuery::default(), 1, limit).is_err());
    }
    for sort in ["name", "size", "modified", "injected"] {
        assert!(
            seek_index(
                &catalog,
                AssetQuery {
                    sort: sort.into(),
                    ..Default::default()
                },
                0,
                240
            )
            .is_err()
        );
    }
    let empty = seek_index(
        &catalog,
        AssetQuery {
            name: "no such fixture".into(),
            ..Default::default()
        },
        u32::MAX,
        240,
    )
    .unwrap();
    assert!(empty.assets.is_empty());
    assert_eq!(empty.start_index, Some(0));
    assert_eq!(empty.total, 0);
    let query = AssetQuery {
        include_missing: false,
        favorites_only: true,
        ..Default::default()
    };
    let page = seek_index(&catalog, query, 5, 20).unwrap();
    assert_eq!(page.start_index, Some(5));
    assert!(
        page.assets
            .iter()
            .all(|asset| asset.favorite && asset.presence == "present")
    );
}
