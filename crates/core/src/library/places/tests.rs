use super::*;
use crate::{catalog::Catalog, library::store};
use rusqlite::params;

#[test]
fn offline_index_disambiguates_names_folds_accents_and_bounds_results() {
    let london = search("London", None).unwrap();
    assert!(
        london
            .matches
            .iter()
            .any(|p| p.id == "places-v1:city:2643743" && p.label.contains("United Kingdom"))
    );
    assert!(
        search("UK", None)
            .unwrap()
            .matches
            .iter()
            .any(|p| p.id == "places-v1:country:GBR")
    );
    assert!(
        search("London UK", None)
            .unwrap()
            .matches
            .iter()
            .any(|p| p.id == "places-v1:city:2643743")
    );
    assert!(
        london
            .matches
            .iter()
            .any(|p| p.id == "places-v1:city:6058560" && p.label.contains("Canada"))
    );
    for name in [
        "Porto",
        "Lisboa",
        "Lisbon",
        "São Paulo",
        "Sao Paulo",
        "Portugal",
    ] {
        assert!(!search(name, None).unwrap().matches.is_empty(), "{name}");
    }
    assert!(search("", None).unwrap().matches.is_empty());
    assert!(search("L", None).unwrap().matches.is_empty());
    assert!(search("an", None).unwrap().more);
    assert_eq!(search("an", None).unwrap().matches.len(), 20);
    assert!(
        search("<script>not-a-place</script>", None)
            .unwrap()
            .matches
            .is_empty()
    );
    assert!(search(&"x".repeat(257), None).is_err());
    assert!(search("", Some("invented")).is_err());
    assert!(
        search("", Some("places-v1:city:2643743"))
            .unwrap()
            .selected
            .is_some()
    );
    let data = dataset().unwrap();
    assert_eq!(data.len(), 34_462);
    for place in data.values() {
        place.summary.bounds.validate().unwrap();
        assert!(place.summary.id.len() <= 64);
    }
}

#[test]
fn city_radius_country_shapes_islands_holes_poles_and_dateline_are_distinct() {
    let london = lookup("places-v1:city:2643743").unwrap();
    assert!(london.contains(51.5074, -0.1278));
    assert!(!london.contains(42.9849, -81.2453));
    // Near the corner of the candidate box, beyond the actual 25 km circle.
    let b = london.summary.bounds;
    assert!(!london.contains(b.north - 0.001, b.east - 0.001));
    let portugal = lookup("places-v1:country:PRT").unwrap();
    assert!(portugal.contains(41.1579, -8.6291));
    assert!(portugal.contains(37.8, -25.5)); // São Miguel interior, Azores
    assert!(!portugal.contains(40.4168, -3.7038)); // Madrid, inside the overall box
    assert!(!portugal.contains(0.0, 0.0));
    let usa = lookup("places-v1:country:USA").unwrap();
    assert!(usa.contains(40.7128, -74.0060));
    assert!(usa.contains(21.3069, -157.8583));
    assert!(!usa.contains(49.2827, -123.1207));
    assert!(
        lookup("places-v1:country:RUS")
            .unwrap()
            .contains(68.0, 175.0)
    );
    assert!(
        lookup("places-v1:country:RUS").unwrap().summary.bounds.west
            > lookup("places-v1:country:RUS").unwrap().summary.bounds.east
    );
    let crossing = [
        Polygon::new(vec![vec![
            [170.0, 0.0],
            [180.0, 0.0],
            [180.0, 10.0],
            [170.0, 10.0],
            [170.0, 0.0],
        ]])
        .unwrap(),
        Polygon::new(vec![vec![
            [-180.0, 0.0],
            [-170.0, 0.0],
            [-170.0, 10.0],
            [-180.0, 10.0],
            [-180.0, 0.0],
        ]])
        .unwrap(),
    ];
    let crossing_bounds = country_bounds(&crossing).unwrap();
    assert_eq!(
        (crossing_bounds.west, crossing_bounds.east),
        (170.0, -170.0)
    );
    for longitude in [175.0, -175.0, -180.0] {
        assert!(
            crossing
                .iter()
                .any(|polygon| polygon.contains(5.0, longitude))
        );
    }
    assert!(!crossing.iter().any(|polygon| polygon.contains(5.0, 0.0)));
    assert!(
        lookup("places-v1:country:ATA")
            .unwrap()
            .contains(-89.0, 0.0)
    );
    let polygon = Polygon::new(vec![
        vec![
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
            [0.0, 0.0],
        ],
        vec![[4.0, 4.0], [6.0, 4.0], [6.0, 6.0], [4.0, 6.0], [4.0, 4.0]],
    ])
    .unwrap();
    assert!(polygon.contains(2.0, 2.0));
    assert!(!polygon.contains(5.0, 5.0));
    assert!(polygon.contains(0.0, 5.0));
    assert!(!london.contains(f64::NAN, 0.0));
    assert!(geometry::city_contains(0.0, -179.99, 0.0, 179.99));
    assert!(geometry::city_bounds(89.99, 12.0).east == 180.0);
}

#[test]
fn country_candidates_never_exclude_any_polygon_bounds() {
    for place in dataset().unwrap().values() {
        let Shape::Country(polygons) = &place.shape else {
            continue;
        };
        let bounds = place.summary.bounds;
        for polygon in polygons {
            for latitude in [polygon.bounds.south, polygon.bounds.north] {
                assert!(
                    latitude >= bounds.south && latitude <= bounds.north,
                    "{}",
                    place.summary.id
                );
            }
            for longitude in [polygon.bounds.west, polygon.bounds.east] {
                assert!(
                    if bounds.west <= bounds.east {
                        longitude >= bounds.west && longitude <= bounds.east
                    } else {
                        longitude >= bounds.west || longitude <= bounds.east
                    },
                    "{} {longitude}",
                    place.summary.id
                );
            }
        }
    }
}

#[test]
fn place_queries_share_paging_buckets_neighbors_and_selection_and_intersect_dates() {
    let owned = tempfile::tempdir().unwrap();
    let catalog = Catalog::open(owned.path().join("Places.stillharbour")).unwrap();
    catalog.call(|db| {
        db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter) VALUES('root','Generated',x'01','Generated','referenced',1,'fixture','fixture','fixture','fixture','offline','{}')", [])?;
        for (index, (latitude,longitude,presence,date)) in [
            (Some(51.5074),Some(-0.1278),"present","2025-10-10T00:00:00"),
            (Some(51.515),Some(-0.11),"present","2025-10-10T23:59:59"),
            (Some(42.9849),Some(-81.2453),"present","2025-10-10T12:00:00"),
            (None,None,"present","2025-10-10T12:00:00"),
            (Some(51.5074),Some(-0.1278),"missing","2025-10-10T12:00:00"),
            (Some(51.5074),Some(-0.1278),"excluded","2025-10-10T12:00:00"),
            (Some(0.0),Some(0.0),"present","2025-10-10T12:00:00"),
            (Some(51.5074),Some(-0.1278),"present","2025-10-11T12:00:00"),
        ].into_iter().enumerate() {
            let id = uuid::Uuid::from_u128(index as u128+1).to_string();
            let raw = serde_json::json!([{"GPSLatitude":latitude,"GPSLongitude":longitude}]).to_string();
            db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan,raw_metadata) VALUES(?1,'root',CAST(?1 AS BLOB),?1,x'','',?1,?1,'jpg',?1,1,'fixture','2026-11-05',?2,'embedded_original','ready','unavailable',?3,1,'fixture',?4)", params![id,date,presence,raw])?;
        }
        Ok(())
    }).unwrap();
    let query = AssetQuery {
        place_id: Some("places-v1:city:2643743".into()),
        name: "10-10-2025".into(),
        ..Default::default()
    };
    let first = store::query(&catalog, query.clone(), None, 1).unwrap();
    assert_eq!(first.total, 2);
    assert_eq!(first.assets.len(), 1);
    let second = store::query(&catalog, query.clone(), first.next_cursor, 1).unwrap();
    assert_eq!(second.assets.len(), 1);
    assert!(second.next_cursor.is_none());
    assert_ne!(first.assets[0].id, second.assets[0].id);
    assert_eq!(
        store::neighbors(&catalog, query.clone(), first.assets[0].id.clone())
            .unwrap()
            .next
            .unwrap()
            .id,
        second.assets[0].id
    );
    assert_eq!(store::buckets(&catalog, query.clone()).unwrap()[0].count, 2);
    assert_eq!(store::freeze(&catalog, query.clone()).unwrap().count, 2);
    assert_eq!(
        store::query(
            &catalog,
            AssetQuery {
                place_id: Some("places-v1:country:GBR".into()),
                ..query.clone()
            },
            None,
            240
        )
        .unwrap()
        .total,
        2
    );
    assert_eq!(
        store::query(
            &catalog,
            AssetQuery {
                include_missing: true,
                ..query.clone()
            },
            None,
            240
        )
        .unwrap()
        .total,
        3
    );
    assert_eq!(
        store::query(
            &catalog,
            AssetQuery {
                name: "".into(),
                ..query.clone()
            },
            None,
            240
        )
        .unwrap()
        .total,
        3
    );
    assert_eq!(
        store::query(
            &catalog,
            AssetQuery {
                place_id: None,
                ..query.clone()
            },
            None,
            240
        )
        .unwrap()
        .total,
        5
    );
    assert!(
        store::query(
            &catalog,
            AssetQuery {
                place_id: Some("untrusted".into()),
                ..query
            },
            None,
            240
        )
        .is_err()
    );
}
