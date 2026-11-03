use super::*;
use rusqlite::params;

fn database() -> Connection {
    database_version(crate::catalog::SCHEMA_VERSION)
}
fn database_version(version: u32) -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF;")
        .unwrap();
    store::migrate_through(&mut db, version).unwrap();
    for (id, base) in [("root", 1), ("other", 0)] {
        db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter) VALUES(?1,?1,x'01',?1,'referenced',?2,'fixture','fixture','fixture','fixture','offline','{}')",params![id,base]).unwrap();
    }
    db
}
fn insert(db: &Connection, id: &str, raw: &str) {
    db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan,raw_metadata) VALUES(?1,'root',CAST(?1 AS BLOB),?1,x'','',?1,?1,'jpg',?1,1,'revision','2026-11-01','2026-11-01','capture','ready','unavailable','present',1,'fixture',?2)",params![id,raw]).unwrap();
}
fn point(db: &Connection, id: &str, lat: f64, lon: f64) {
    insert(
        db,
        id,
        &serde_json::json!([{"GPSLatitude":lat,"GPSLongitude":lon}]).to_string(),
    );
}
fn world() -> MapBounds {
    MapBounds {
        west: -180.0,
        south: -90.0,
        east: 180.0,
        north: 90.0,
    }
}
fn view(db: &Connection, query: AssetQuery, bounds: MapBounds, zoom: u8) -> MapView {
    let mut scoped = query.clone();
    scoped.map_bounds = Some(bounds);
    let (all_where, all_values) = store::query_where(&query).unwrap();
    let (sql, values) = store::query_where(&scoped).unwrap();
    query_connection(db, &all_where, all_values, &sql, values, zoom).unwrap()
}

#[test]
fn zero_missing_invalid_and_signed_coordinates_keep_distinct_membership() {
    let db = database();
    point(&db, "zero", 0.0, 0.0);
    point(&db, "south-west", -33.5, -70.7);
    for (id, raw) in [
        ("missing", "[]"),
        ("partial", r#"[{"GPSLatitude":0}]"#),
        ("text", r#"[{"GPSLatitude":"zero","GPSLongitude":"zero"}]"#),
        ("boolean", r#"[{"GPSLatitude":true,"GPSLongitude":false}]"#),
        ("invalid", r#"[{"GPSLatitude":91,"GPSLongitude":0}]"#),
        ("infinite", r#"[{"GPSLatitude":9e999,"GPSLongitude":0}]"#),
        ("malformed", "not JSON"),
    ] {
        insert(&db, id, raw);
    }
    let result = view(&db, AssetQuery::default(), world(), 4);
    assert_eq!(
        (result.located, result.unavailable, result.in_view),
        (2, 7, 2)
    );
    let zero = view(
        &db,
        AssetQuery::default(),
        MapBounds {
            west: 0.0,
            east: 0.0,
            south: 0.0,
            north: 0.0,
        },
        16,
    );
    assert_eq!(zero.in_view, 1);
    assert_eq!(zero.clusters[0].representative.id, "zero");
    assert_eq!(zero.clusters[0].latitude, 0.0);
}

#[test]
fn antimeridian_edges_and_float32_candidates_use_exact_coordinates() {
    let db = database();
    for (id, lon) in [
        ("east", 179.5),
        ("west", -179.5),
        ("edge", 180.0),
        ("middle", 0.0),
        ("outside", 179.5000001),
    ] {
        point(&db, id, 5.0, lon);
    }
    let crossing = view(
        &db,
        AssetQuery::default(),
        MapBounds {
            west: 179.0,
            east: -179.0,
            south: 0.0,
            north: 10.0,
        },
        16,
    );
    assert_eq!(crossing.in_view, 4);
    let exact = view(
        &db,
        AssetQuery::default(),
        MapBounds {
            west: 179.5,
            east: 179.5,
            south: 5.0,
            north: 5.0,
        },
        16,
    );
    assert_eq!(exact.in_view, 1);
    assert_eq!(exact.clusters[0].representative.id, "east");
    let edge = view(
        &db,
        AssetQuery::default(),
        MapBounds {
            west: 180.0,
            east: 180.0,
            south: 5.0,
            north: 5.0,
        },
        16,
    );
    assert_eq!(edge.in_view, 1);
    assert_eq!(edge.clusters[0].longitude, -180.0);
    assert_eq!(view(&db, AssetQuery::default(), world(), 2).in_view, 5);
}

#[test]
fn dense_and_identical_locations_are_bounded_without_losing_counts() {
    let db = database();
    db.execute_batch("BEGIN").unwrap();
    for i in 0..2000 {
        point(&db, &format!("same-{i:04}"), 40.0, 12.0);
    }
    for i in 0..2000 {
        point(
            &db,
            &format!("spread-{i:04}"),
            -80.0 + (i % 150) as f64,
            -170.0 + (i % 340) as f64,
        );
    }
    db.execute_batch("COMMIT").unwrap();
    let result = view(&db, AssetQuery::default(), world(), 16);
    assert!(result.clusters.len() <= MAX_CLUSTERS);
    assert!(result.cluster_zoom < 16);
    assert_eq!(result.clusters.iter().map(|c| c.count).sum::<u32>(), 4000);
    let exact = view(
        &db,
        AssetQuery::default(),
        MapBounds {
            west: 12.0,
            east: 12.0,
            south: 40.0,
            north: 40.0,
        },
        16,
    );
    assert_eq!(exact.clusters.len(), 1);
    assert_eq!(exact.clusters[0].count, 2000);
}

#[test]
fn index_reconciles_revision_clear_delete_and_exclusions_without_touching_raw_history() {
    let db = database();
    point(&db, "changing", 20.0, 30.0);
    let raw: String = db
        .query_row(
            "SELECT raw_metadata FROM assets WHERE id='changing'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let before = db.total_changes();
    assert_eq!(view(&db, AssetQuery::default(), world(), 1).located, 1);
    assert_eq!(db.total_changes(), before);
    assert_eq!(
        db.query_row(
            "SELECT raw_metadata FROM assets WHERE id='changing'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        raw
    );
    db.execute(
        "UPDATE assets SET raw_metadata=NULL WHERE id='changing'",
        [],
    )
    .unwrap();
    assert_eq!(view(&db, AssetQuery::default(), world(), 1).located, 0);
    db.execute(
        "UPDATE assets SET raw_metadata=? WHERE id='changing'",
        [raw],
    )
    .unwrap();
    db.execute(
        "UPDATE assets SET presence='excluded' WHERE id='changing'",
        [],
    )
    .unwrap();
    assert_eq!(view(&db, AssetQuery::default(), world(), 1).located, 0);
    db.execute("DELETE FROM assets WHERE id='changing'", [])
        .unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM asset_spatial", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT rtreecheck('asset_spatial')", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn root_date_tag_filters_and_no_usable_location_share_library_membership() {
    let db = database();
    point(&db, "keep", 51.0, 0.0);
    point(&db, "other-root", 51.0, 0.0);
    point(&db, "old", 51.0, 0.0);
    insert(&db, "absent", "[]");
    db.execute(
        "UPDATE assets SET root_id='other' WHERE id='other-root'",
        [],
    )
    .unwrap();
    db.execute("UPDATE assets SET date='2020-01-01' WHERE id='old'", [])
        .unwrap();
    db.execute(
        "UPDATE assets SET tags='[\"trip\"]' WHERE id IN ('keep','absent')",
        [],
    )
    .unwrap();
    let mut query = AssetQuery {
        root_id: Some("root".into()),
        from_date: "2026-01-01".into(),
        tag: "trip".into(),
        ..AssetQuery::default()
    };
    let result = view(&db, query.clone(), world(), 8);
    assert_eq!((result.located, result.unavailable), (1, 1));
    query.location = Some(LocationFilter::Unavailable);
    let (sql, values) = store::query_where(&query).unwrap();
    let id: String = db
        .query_row(
            &format!("SELECT id FROM assets WHERE {sql}"),
            params_from_iter(values),
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(id, "absent");
}

#[test]
fn malformed_bounds_fail_before_sql_and_do_not_accept_unknown_ipc_fields() {
    for bounds in [
        MapBounds {
            north: f64::NAN,
            ..world()
        },
        MapBounds {
            west: 181.0,
            ..world()
        },
        MapBounds {
            south: 30.0,
            north: 20.0,
            ..world()
        },
    ] {
        assert!(bounds.validate().is_err());
    }
    assert!(
        serde_json::from_str::<MapBounds>(
            r#"{"west":0,"east":1,"south":0,"north":1,"url":"untrusted"}"#
        )
        .is_err()
    );
}

#[test]
fn cluster_selection_uses_exact_cells_in_the_normal_library_predicate() {
    let db = database();
    for (id, lat, lon) in [
        ("zero", 0.0, 0.0),
        ("zero-again", 0.0, 0.0),
        ("left", 0.0, -0.000001),
        ("south", -0.000001, 0.0),
    ] {
        point(&db, id, lat, lon);
    }
    let result = view(&db, AssetQuery::default(), world(), 16);
    let cluster = result.clusters.iter().find(|c| c.count == 2).unwrap();
    let query = AssetQuery {
        map_bounds: Some(world()),
        map_cell: Some(cluster.cell),
        ..AssetQuery::default()
    };
    let (sql, values) = store::query_where(&query).unwrap();
    let count: u32 = db
        .query_row(
            &format!("SELECT count(*) FROM assets WHERE {sql}"),
            params_from_iter(values),
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, cluster.count);
    for cell in [
        MapCell {
            zoom: 17,
            x: 0,
            y: 0,
        },
        MapCell {
            zoom: 1,
            x: 2,
            y: 0,
        },
    ] {
        assert!(
            store::query_where(&AssetQuery {
                map_cell: Some(cell),
                ..AssetQuery::default()
            })
            .is_err()
        );
    }
}

#[test]
fn jsonq_numeric_text_is_admitted_but_arbitrary_text_booleans_and_nonfinite_values_are_not() {
    let db = database();
    insert(
        &db,
        "quoted-zero",
        r#"[{"GPSLatitude":"0","GPSLongitude":"0"}]"#,
    );
    insert(
        &db,
        "quoted-signed",
        r#"[{"GPSLatitude":"-3.35e1","GPSLongitude":"-70.7"}]"#,
    );
    for (i, value) in [
        "true",
        "false",
        "NaN",
        "Infinity",
        "9e999",
        "01",
        "{\"n\":1}",
        "null",
        "",
        "not a number",
    ]
    .into_iter()
    .enumerate()
    {
        insert(
            &db,
            &format!("invalid-{i}"),
            &serde_json::json!([{"GPSLatitude":value,"GPSLongitude":"0"}]).to_string(),
        );
    }
    let result = view(&db, AssetQuery::default(), world(), 4);
    assert_eq!((result.located, result.unavailable), (2, 10));
    assert!(
        result
            .clusters
            .iter()
            .any(|c| c.latitude == -33.5 && c.longitude == -70.7)
    );
}

#[test]
fn jsonq_upgrade_backfills_old_observations_without_rewriting_raw_metadata() {
    let mut db = database_version(14);
    let raw = r#"[{"GPSLatitude":"0","GPSLongitude":"0"}]"#;
    insert(&db, "previous-scan", raw);
    assert_eq!(view(&db, AssetQuery::default(), world(), 0).located, 0);
    store::migrate(&mut db).unwrap();
    assert_eq!(view(&db, AssetQuery::default(), world(), 0).located, 1);
    assert_eq!(
        db.query_row(
            "SELECT raw_metadata FROM assets WHERE id='previous-scan'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        raw
    );
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        crate::catalog::SCHEMA_VERSION
    );
}

#[test]
#[cfg(all(target_os = "macos", feature = "fixture-probes"))]
fn generated_exif_composite_coordinates_preserve_hemisphere_signs() {
    let fixture = crate::library_fixture::LibraryFixture::generate_geographic().unwrap();
    let helper =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/exiftool/exiftool");
    for (name, expected) in [("Zero", (0.0, 0.0)), ("South", (-33.5, -70.7))] {
        let path = fixture.path().join(format!("{name}.jpg"));
        let output = std::process::Command::new("/usr/bin/perl")
            .arg(&helper)
            .args(crate::backup::metadata_arguments(&path).unwrap())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        // The pinned helper deliberately uses JSONQ to retain numerical text.
        assert_eq!(
            raw[0]["GPSLatitude"]
                .as_str()
                .and_then(|v| v.parse::<f64>().ok()),
            Some(expected.0),
            "{raw}"
        );
        assert_eq!(
            raw[0]["GPSLongitude"]
                .as_str()
                .and_then(|v| v.parse::<f64>().ok()),
            Some(expected.1),
            "{raw}"
        );
    }
    assert_eq!(fixture.verify_unchanged().unwrap(), 9);
}
