use super::day;
use crate::{catalog::Catalog, library::model::AssetQuery, library::store};
use rusqlite::params;

#[test]
fn calendar_search_accepts_only_complete_valid_days() {
    for (input, expected) in [
        ("10-10-2025", "2025-10-10"),
        ("03-04-2025", "2025-04-03"),
        ("2025-04-03", "2025-04-03"),
        (" 29-02-2024 ", "2024-02-29"),
        ("29-02-1964", "1964-02-29"),
        ("31-12-9999", "9999-12-31"),
    ] {
        assert_eq!(day(input).as_deref(), Some(expected), "{input}");
    }
    for input in [
        "",
        "10-10",
        "1-10-2025",
        "10-1-2025",
        "2025-1-01",
        "2025",
        "31-02-2025",
        "29-02-2025",
        "00-10-2025",
        "10-00-2025",
        "10-13-2025",
        "10-10-0000",
        "10/10/2025",
        "photo-10-10-2025.jpg",
        "2025-10-10.jpg",
        "１０-１０-２０２５",
        "2025-10-10T00:00:00",
        "2025-10-10 OR 1=1",
        "%_literal",
    ] {
        assert_eq!(day(input), None, "{input}");
    }
}

fn fixture() -> (tempfile::TempDir, Catalog) {
    let owned = tempfile::tempdir().unwrap();
    let catalog = Catalog::open(owned.path().join("Search.stillharbour")).unwrap();
    catalog.call(|db| {
        for (root, base) in [("root", 1), ("other", 0)] {
            db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter) VALUES(?1,?1,x'01',?1,'referenced',?2,'fixture','fixture','fixture','fixture','offline','{}')", params![root,base])?;
        }
        for (index, (name, root, date, source, offset, presence)) in [
            ("early.jpg", "root", "2025-10-10T00:00:00", "embedded_original", Some("+14:00"), "present"),
            ("late.jpg", "root", "2025-10-10T23:59:59.9999", "embedded_created", Some("-12:00"), "present"),
            ("fallback.jpg", "other", "2025-10-10T12:00:00", "filesystem_modified", None, "present"),
            ("10-10-2025.jpg", "root", "2025-10-11T00:00:00", "embedded_original", None, "present"),
            ("previous.jpg", "root", "2025-10-09T23:59:59.9999", "embedded_original", None, "present"),
            ("undated.jpg", "root", "", "unavailable", None, "present"),
            ("missing.jpg", "root", "2025-10-10T12:00:00", "embedded_original", None, "missing"),
            ("excluded.jpg", "root", "2025-10-10T12:00:00", "embedded_original", None, "excluded"),
            ("leap.jpg", "root", "2024-02-29T23:59:59", "embedded_original", None, "present"),
            ("31-02-2025.jpg", "root", "", "unavailable", None, "present"),
            ("percent%_literal.jpg", "root", "", "unavailable", None, "present"),
        ].into_iter().enumerate() {
            let id = uuid::Uuid::from_u128(index as u128 + 1).to_string();
            db.execute(r#"INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,offset,metadata_state,preview_state,presence,is_media,seen_scan,favorite,tags) VALUES(?1,?2,CAST(?1 AS BLOB),?3,x'','',?3,?3,'jpg',?1,1,'fixture','2026-11-05T00:00:00Z',?4,?5,?6,'ready','unavailable',?7,1,'fixture',?8,'["Holiday"]')"#, params![id, root, name, date, source, offset, presence, name == "late.jpg"])?;
        }
        Ok(())
    }).unwrap();
    (owned, catalog)
}

#[test]
fn calendar_search_pages_buckets_neighbors_and_frozen_selection_agree() {
    let (_owned, catalog) = fixture();
    for name in ["10-10-2025", "2025-10-10", " 10-10-2025 "] {
        let query = AssetQuery {
            name: name.into(),
            ..Default::default()
        };
        let first = store::query(&catalog, query.clone(), None, 1).unwrap();
        assert_eq!(first.total, 3);
        assert_eq!(first.assets[0].name, "late.jpg");
        assert_eq!(first.assets[0].offset.as_deref(), Some("-12:00"));
        let next = store::neighbors(&catalog, query.clone(), first.assets[0].id.clone()).unwrap();
        assert!(next.previous.is_none());
        assert_eq!(next.next.unwrap().name, "fallback.jpg");
        let second = store::query(&catalog, query.clone(), first.next_cursor, 1).unwrap();
        assert_eq!(second.assets[0].name, "fallback.jpg");
        let third = store::query(&catalog, query.clone(), second.next_cursor, 1).unwrap();
        assert_eq!(third.assets[0].name, "early.jpg");
        assert_eq!(third.assets[0].offset.as_deref(), Some("+14:00"));
        assert!(third.next_cursor.is_none());
        let buckets = store::buckets(&catalog, query.clone()).unwrap();
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].year, "2025");
        assert_eq!(buckets[0].count, 3);
        assert_eq!(buckets[0].months[0].period, "2025-10");
        let seek = store::seek(&catalog, query.clone(), "2025".into(), 1).unwrap();
        assert_eq!(seek.assets[0].name, "late.jpg");
        let selection = store::freeze(&catalog, query).unwrap();
        assert_eq!(selection.count, 3);
        let names = catalog.call(move |db| {
            Ok(db.prepare("SELECT a.name FROM assets a JOIN selection_members s ON a.id=s.asset_id WHERE s.selection_id=? ORDER BY a.name")?
                .query_map([selection.id], |row| row.get::<_,String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        }).unwrap();
        assert_eq!(names, ["early.jpg", "fallback.jpg", "late.jpg"]);
    }
}

#[test]
fn calendar_search_intersects_filters_and_preserves_literal_filename_search() {
    let (_owned, catalog) = fixture();
    let query = AssetQuery {
        name: "10-10-2025".into(),
        ..Default::default()
    };
    for sort in ["capture", "name", "size", "modified", "added"] {
        let page = store::query(
            &catalog,
            AssetQuery {
                sort: sort.into(),
                ..query.clone()
            },
            None,
            240,
        )
        .unwrap();
        assert_eq!(page.total, 3, "{sort}");
        assert_eq!(page.assets.len(), 3, "{sort}");
    }
    for (scoped, expected) in [
        (
            AssetQuery {
                root_id: Some("root".into()),
                ..query.clone()
            },
            2,
        ),
        (
            AssetQuery {
                root_id: Some("other".into()),
                ..query.clone()
            },
            1,
        ),
        (
            AssetQuery {
                favorites_only: true,
                ..query.clone()
            },
            1,
        ),
        (
            AssetQuery {
                tag: "Holiday".into(),
                ..query.clone()
            },
            3,
        ),
        (
            AssetQuery {
                include_missing: true,
                ..query.clone()
            },
            4,
        ),
        (
            AssetQuery {
                from_date: "2025-10-11".into(),
                ..query.clone()
            },
            0,
        ),
        (
            AssetQuery {
                through_date: "2025-10-09".into(),
                ..query.clone()
            },
            0,
        ),
    ] {
        let page = store::query(&catalog, scoped.clone(), None, 240).unwrap();
        assert_eq!(page.total, expected, "{scoped:?}");
        assert_eq!(page.assets.len(), expected as usize, "{scoped:?}");
        assert_eq!(
            store::buckets(&catalog, scoped)
                .unwrap()
                .iter()
                .map(|bucket| bucket.count)
                .sum::<u32>(),
            expected
        );
    }
    for (name, expected) in [
        ("29-02-2024", vec!["leap.jpg"]),
        ("12-10-2025", vec![]),
        ("31-02-2025", vec!["31-02-2025.jpg"]),
        ("10-10-2025.jpg", vec!["10-10-2025.jpg"]),
        ("%_literal", vec!["percent%_literal.jpg"]),
        ("early", vec!["early.jpg"]),
    ] {
        let query = AssetQuery {
            name: name.into(),
            ..Default::default()
        };
        let page = store::query(&catalog, query, None, 240).unwrap();
        assert_eq!(page.total as usize, expected.len(), "{name}");
        assert_eq!(
            page.assets
                .iter()
                .map(|asset| asset.name.as_str())
                .collect::<Vec<_>>(),
            expected,
            "{name}"
        );
    }
}
