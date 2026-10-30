//! P0 fixtures and measurement, not a replacement catalog path or production optimization.
use super::{model::*, native, store, workers};
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
    filesystem, performance,
};
use rusqlite::params;
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
    time::Instant,
};

pub fn sqlite_info(catalog: &Catalog) -> Result<Value> {
    catalog.call(|db| Ok(json!({
        "version": db.query_row("SELECT sqlite_version()", [], |r| r.get::<_,String>(0))?,
        "sourceId": db.query_row("SELECT sqlite_source_id()", [], |r| r.get::<_,String>(0))?,
        "compileOptions": db.prepare("PRAGMA compile_options")?.query_map([], |r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?,
        "journalMode":db.query_row("PRAGMA journal_mode",[],|r|r.get::<_,String>(0))?,
        "synchronous":db.query_row("PRAGMA synchronous",[],|r|r.get::<_,u32>(0))?,
        "pageSize":db.query_row("PRAGMA page_size",[],|r|r.get::<_,u32>(0))?,
        "schema":db.query_row("PRAGMA user_version",[],|r|r.get::<_,u32>(0))?
    })))
}

pub fn seed_catalog(catalog: &Catalog, parent: &Path, rows: u32) -> Result<Value> {
    if !matches!(rows, 10_000 | 100_000 | 1_000_000) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Choose a documented P0 fixture size.",
        ));
    }
    let mut roots = Vec::new();
    for index in 0..2 {
        let path = parent.join(format!("synthetic-root-{index}"));
        std::fs::create_dir(&path)?;
        let handle = native::open_root(&path, None)?;
        let id = store::add_root(
            catalog,
            path.clone(),
            filesystem::identity(&handle)?,
            native::volume_token(&handle)?,
            "Synthetic metadata only; no media originals".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )?;
        // These are freshly created, empty, test-owned roots. Retain them at a
        // different path so the registered roots are genuinely unavailable.
        // A flag alone lets status reconnect/scan the empty directory and mark
        // every synthetic observation missing, invalidating scale measurements.
        if std::fs::read_dir(&path)?.next().is_some() {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Synthetic fixture root is not empty.",
            ));
        }
        filesystem::publish_noreplace(
            &path,
            &parent.join(format!("retained-synthetic-root-{index}")),
        )?;
        catalog.call({
            let id = id.clone();
            move |db| {
                db.execute("UPDATE roots SET availability='offline' WHERE id=?", [id])?;
                Ok(())
            }
        })?;
        roots.push(id);
    }
    let ids = roots.clone();
    catalog.call(move |db| {
        let tx=db.transaction()?;
        tx.execute(r#"WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<?1)
        INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,modified_at,added_at,date,date_source,metadata_state,preview_state,preview_error,presence,is_media,seen_scan,width,height,tags)
        SELECT printf('70000000-0000-4000-8000-%012d',x),CASE WHEN x%5=0 THEN ?3 ELSE ?2 END,
        CAST(printf('fixture-%07d',x) AS BLOB),printf('fixture-%07d',x),X'','','Synthetic '||printf('%07d',x),'Synthetic',
        CASE x%10 WHEN 0 THEN 'mp4' WHEN 1 THEN 'mov' WHEN 2 THEN 'heic' WHEN 3 THEN 'dng' WHEN 4 THEN 'avif' WHEN 5 THEN 'tiff' WHEN 6 THEN 'gif' WHEN 7 THEN 'png' WHEN 8 THEN 'webp' ELSE 'jpg' END,
        CAST(x AS TEXT),CASE x%10 WHEN 0 THEN 65536 WHEN 1 THEN 8589934592 ELSE 1024+x%30000000 END,
        printf('%064d',x),'2026-10-01T00:00:00Z','2026-10-01T00:00:00Z',
        CASE WHEN x%20=0 THEN '' WHEN x%4=0 THEN '2020-06-15T12:00:00' ELSE printf('%04d-%02d-%02dT12:00:00',1960+x%65,1+x%12,1+x%28) END,
        CASE WHEN x%20=0 THEN 'undated' ELSE 'generated_capture' END,'ready','unavailable','Synthetic catalog record; no source media exists','present',1,'synthetic-only',
        CASE x%7 WHEN 0 THEN 12000 WHEN 1 THEN 40 ELSE 600 END,
        CASE x%7 WHEN 0 THEN 40 WHEN 1 THEN 12000 ELSE 400 END,
        CASE WHEN x%3=0 THEN '["Fixture tag","Dense"]' ELSE '[]' END FROM n"#, params![rows,roots[0],roots[1]])?;
        tx.execute("UPDATE library_state SET revision=revision+1",[])?;
        tx.commit()?;
        // Match newly seeded observations to existing indexes; no index/schema/durability change.
        db.execute_batch("ANALYZE; PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    })?;
    Ok(
        json!({"rows":rows,"rootIds":ids,"synthetic":true,"seedVersion":2,"seed":20261028,"offlineRoots":2,"unknownDates":rows/20,"denseEqualTimestampDay":rows/5,"videoRows":rows/5,"largeVideoLogicalBytes":8589934592u64}),
    )
}

fn elapsed(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
fn distribution(samples: &[f64]) -> Value {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let p = |q: f64| {
        sorted[((q * sorted.len() as f64).ceil() as usize)
            .saturating_sub(1)
            .min(sorted.len() - 1)]
    };
    json!({"n":sorted.len(),"p50Ms":p(0.5),"p95Ms":p(0.95),"p99Ms":p(0.99),"minMs":sorted[0],"maxMs":sorted[sorted.len()-1]})
}
fn measure<T>(n: usize, mut work: impl FnMut() -> Result<T>) -> Result<Value> {
    let mut samples = Vec::with_capacity(n);
    for _ in 0..n {
        let start = Instant::now();
        let _ = work()?;
        samples.push(elapsed(start));
    }
    Ok(distribution(&samples))
}

/// Current filtered reads of an existing disposable fixture. Never scans sources.
pub fn measure_filtered(path: &Path, rows: u32) -> Result<Value> {
    let catalog = Catalog::open(path.to_owned())?;
    let actual: u32 =
        catalog.call(|db| Ok(db.query_row("SELECT count(*) FROM assets", [], |r| r.get(0))?))?;
    if actual != rows {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Fixture row count changed.",
        ));
    }
    let mut results = Vec::new();
    for (label, name) in [
        ("none", "Generated"),
        ("narrow", "Synthetic 0000"),
        ("broad", "Synthetic"),
        ("lateBroad", "Synthetic 099"),
        ("lateNarrow", "Synthetic 09999"),
    ] {
        let query = AssetQuery {
            name: name.into(),
            ..Default::default()
        };
        let mut samples = Vec::new();
        let mut matched = 0;
        for _ in 0..3 {
            catalog.cached(|_, cache| {
                cache.reset_for_probe();
                Ok(())
            })?;
            let start = Instant::now();
            let page = store::query(&catalog, query.clone(), None, 240)?;
            let page_ms = elapsed(start);
            matched = page.total;
            let start = Instant::now();
            let buckets = store::buckets(&catalog, query.clone())?;
            let bucket_ms = elapsed(start);
            assert_eq!(buckets.iter().map(|b| b.count).sum::<u32>(), page.total);
            let start = Instant::now();
            store::buckets(&catalog, query.clone())?;
            samples.push(
                json!({"pageMs":page_ms,"bucketsMs":bucket_ms,"warmBucketsMs":elapsed(start)}),
            );
        }
        let mut timeline_first = Vec::new();
        for _ in 0..3 {
            catalog.cached(|_, cache| {
                cache.reset_for_probe();
                Ok(())
            })?;
            let start = Instant::now();
            let buckets = store::buckets(&catalog, query.clone())?;
            let buckets_ms = elapsed(start);
            let start = Instant::now();
            let page = store::query(&catalog, query.clone(), None, 240)?;
            let page_ms = elapsed(start);
            assert_eq!(page.total, matched);
            assert_eq!(buckets.iter().map(|b| b.count).sum::<u32>(), matched);
            timeline_first.push(json!({"pageMs":page_ms,"bucketsMs":buckets_ms}));
        }
        let (where_sql, values) = store::query_where(&query)?;
        let plan_query = query.clone();
        let plans = catalog.call(move |db| {
            let sql = format!("EXPLAIN QUERY PLAN SELECT CASE WHEN length(date)<10 THEN 'Undated' ELSE substr(date,1,7) END AS period,count(*) FROM assets WHERE {where_sql} GROUP BY period ORDER BY (period='Undated'),period DESC");
            let fallback = db.prepare(&sql)?.query_map(rusqlite::params_from_iter(values.clone()), |r| r.get::<_,String>(3))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let (where_sql, values) = store::narrow_name_scope(db, &plan_query, where_sql, values, Some(matched))?;
            let sql = format!("EXPLAIN QUERY PLAN SELECT CASE WHEN length(date)<10 THEN 'Undated' ELSE substr(date,1,7) END AS period,count(*) FROM assets WHERE {where_sql} GROUP BY period ORDER BY (period='Undated'),period DESC");
            let buckets = db.prepare(&sql)?.query_map(rusqlite::params_from_iter(values.clone()), |r| r.get::<_,String>(3))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let sql = format!("EXPLAIN QUERY PLAN SELECT {} FROM assets WHERE {where_sql} ORDER BY date DESC,id DESC LIMIT 241", store::ASSET_COLUMNS);
            let page = db.prepare(&sql)?.query_map(rusqlite::params_from_iter(values), |r| r.get::<_,String>(3))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((fallback, buckets, page))
        })?;
        let name = name.to_owned();
        let candidate_probe = catalog.call(move |db| {
            let sql = "SELECT rowid FROM assets WHERE name LIKE ? ESCAPE '\\' LIMIT ?";
            let pattern = format!("%{}%", store::escape_like(&name));
            let plan = db
                .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?
                .query_map(params![&pattern, store::NAME_CANDIDATE_LIMIT + 1], |r| {
                    r.get::<_, String>(3)
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let mut samples = Vec::new();
            let mut candidates = 0;
            for _ in 0..3 {
                let start = Instant::now();
                candidates = db
                    .prepare(sql)?
                    .query_map(params![&pattern, store::NAME_CANDIDATE_LIMIT + 1], |r| {
                        r.get::<_, i64>(0)
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?
                    .len();
                samples.push(elapsed(start));
            }
            Ok(json!({"plan":plan,"retained":candidates,"samplesMs":samples}))
        })?;
        let summary_probe = measure_name_summaries(&catalog, query.name)?;
        results.push(json!({"scope":label,"matched":matched,"samples":samples,"timelineFirst":timeline_first,"bucketPlan":plans.0,"pageFirstBucketPlan":plans.1,"pagePlan":plans.2,"candidateProbe":candidate_probe,"summaryProbe":summary_probe}));
    }
    Ok(json!({"sqlite":sqlite_info(&catalog)?,"scenarios":results}))
}

/// Compare predicate order on name-only owned-fixture summaries. These reference
/// statements never affect production SQL or populate the derived summary cache.
fn measure_name_summaries(catalog: &Catalog, name: String) -> Result<Value> {
    let query = AssetQuery {
        name,
        ..Default::default()
    };
    let (current, values) = store::query_where(&query)?;
    let (residual, _) = store::query_where(&AssetQuery::default())?;
    let legacy = format!("{residual} AND name LIKE ? ESCAPE '\\'");
    catalog.call(move |db| {
        let mut result = Vec::new();
        let mut reference = None;
        for (label, predicate) in [("legacyNameLast", legacy), ("currentNameEarly", current)] {
            let count_sql = format!("SELECT count(*) FROM assets WHERE {predicate}");
            let bucket_sql = format!("SELECT CASE WHEN length(date)<10 THEN 'Undated' ELSE substr(date,1,7) END AS period,count(*) FROM assets WHERE {predicate} GROUP BY period ORDER BY (period='Undated'),period DESC");
            let mut plans = Vec::new();
            let mut samples = Vec::new();
            for sql in [&count_sql, &bucket_sql] {
                plans.push(db.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?.query_map(rusqlite::params_from_iter(&values), |r| r.get::<_,String>(3))?.collect::<rusqlite::Result<Vec<_>>>()?);
            }
            for _ in 0..3 {
                db.execute_batch("PRAGMA shrink_memory")?;
                let before = performance::pager_counters(db).unwrap();
                let start = Instant::now();
                let mut count_statement = db.prepare(&count_sql)?;
                let count: u32 = count_statement.query_row(rusqlite::params_from_iter(&values), |r|r.get(0))?;
                let count_ms = elapsed(start);
                let count_steps = count_statement.get_status(rusqlite::StatementStatus::VmStep);
                let after_count = performance::pager_counters(db).unwrap();
                db.execute_batch("PRAGMA shrink_memory")?;
                let start = Instant::now();
                let mut bucket_statement = db.prepare(&bucket_sql)?;
                let buckets = bucket_statement.query_map(rusqlite::params_from_iter(&values), |r|Ok((r.get::<_,String>(0)?,r.get::<_,u32>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
                let bucket_ms = elapsed(start);
                let bucket_steps = bucket_statement.get_status(rusqlite::StatementStatus::VmStep);
                let after = performance::pager_counters(db).unwrap();
                let observed = (count, buckets);
                if let Some(reference) = &reference { assert_eq!(&observed, reference); } else { reference = Some(observed); }
                samples.push(json!({"countMs":count_ms,"bucketsMs":bucket_ms,"countVmSteps":count_steps,"bucketVmSteps":bucket_steps,"countLogicalBytes":(after_count.0-before.0) as u64*before.2,"bucketLogicalBytes":(after.0-after_count.0) as u64*before.2}));
            }
            result.push(json!({"variant":label,"plans":plans,"samples":samples}));
        }
        Ok(json!(result))
    })
}

/// P1 focused release measurements. All cache resets affect derived reads of
/// this disposable catalog only; no OS cache purge or production cache eviction.
pub fn measure_p1(path: &Path, fixture: &Value) -> Result<Value> {
    measure_catalog_pages(path, fixture, false)
}

/// P4 retains each bounded trace segment between timed groups, outside stopwatch samples.
pub fn measure_p4(path: &Path, fixture: &Value) -> Result<Value> {
    measure_catalog_pages(path, fixture, true)
}

fn measure_catalog_pages(path: &Path, fixture: &Value, segmented: bool) -> Result<Value> {
    let mut segments = Vec::new();
    macro_rules! sample {
        ($count:expr, $work:expr) => {{
            let result = measure($count, $work);
            if segmented {
                segments.push(performance::drain());
            }
            result
        }};
    }
    performance::enable();
    let catalog = Catalog::open(path.to_owned())?;
    let sqlite = sqlite_info(&catalog)?;
    let query = AssetQuery::default();
    let first = store::query(&catalog, query.clone(), None, 240)?;
    assert_eq!(first.total, fixture["rows"].as_u64().unwrap() as u32);
    let last = catalog.call(|db| Ok(db.query_row("SELECT id FROM assets INDEXED BY assets_visible_timeline WHERE is_media=1 AND presence NOT IN ('excluded','missing') ORDER BY date,id LIMIT 1",[],|r|r.get::<_,String>(0))?))?;
    let reset = || {
        catalog.cached(|_, cache| {
            cache.reset_for_probe();
            Ok(())
        })
    };
    let mut metrics = serde_json::Map::new();
    metrics.insert(
        "firstPage".into(),
        sample!(100, || store::query(&catalog, query.clone(), None, 240))?,
    );
    metrics.insert(
        "nextPage".into(),
        sample!(100, || {
            store::query(&catalog, query.clone(), first.next_cursor.clone(), 240)
        })?,
    );
    metrics.insert(
        "firstBoundaryNeighbors".into(),
        sample!(100, || {
            store::neighbors(&catalog, query.clone(), first.assets[0].id.clone())
        })?,
    );
    metrics.insert(
        "lastBoundaryNeighbors".into(),
        sample!(100, || {
            store::neighbors(&catalog, query.clone(), last.clone())
        })?,
    );
    metrics.insert(
        "timelineBucketsUncached".into(),
        sample!(10, || {
            reset()?;
            store::buckets(&catalog, query.clone())
        })?,
    );
    metrics.insert(
        "timelineBucketsCached".into(),
        sample!(100, || store::buckets(&catalog, query.clone()))?,
    );
    let tagged = AssetQuery {
        tag: "Fixture tag".into(),
        ..Default::default()
    };
    metrics.insert(
        "tagPageUncachedCount".into(),
        sample!(10, || {
            reset()?;
            store::query(&catalog, tagged.clone(), None, 240)
        })?,
    );
    metrics.insert(
        "tagPageCachedCount".into(),
        sample!(100, || store::query(&catalog, tagged.clone(), None, 240))?,
    );
    let mut seed = 20261028u64;
    metrics.insert(
        "randomDateSeek".into(),
        sample!(100, || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            store::seek(
                &catalog,
                query.clone(),
                (1960 + (seed >> 32) % 65).to_string(),
                240,
            )
        })?,
    );
    reset()?;
    catalog.call(|db| {
        db.execute_batch("PRAGMA shrink_memory")?;
        Ok(())
    })?;
    let before = catalog.call(|db| Ok(performance::pager_counters(db).unwrap()))?;
    let neighbors = store::neighbors(&catalog, query.clone(), first.assets[0].id.clone())?;
    assert!(neighbors.previous.is_none() && neighbors.next.is_some());
    let after = catalog.call(|db| Ok(performance::pager_counters(db).unwrap()))?;
    let anchor = first.assets[0].clone();
    let plans = catalog.call(move |db| {
        let mut plans = Vec::new();
        let (where_sql,_) = store::query_where(&AssetQuery::default())?;
        let source = store::ordered_source(&AssetQuery::default());
        for (comparison,order) in [("<","DESC"),(">","ASC")] {
            let sql = format!("EXPLAIN QUERY PLAN SELECT {} FROM {source} WHERE {where_sql} AND (date,id){comparison}(?,?) ORDER BY date {order},id {order} LIMIT 1",store::ASSET_COLUMNS);
            let plan = db.prepare(&sql)?.query_map(params![anchor.date,anchor.id],|r|r.get::<_,String>(3))?.collect::<rusqlite::Result<Vec<_>>>()?;
            assert!(plan.iter().all(|line|!line.contains("TEMP B-TREE")));
            plans.push(json!({"direction":comparison,"plan":plan}));
        }
        Ok(plans)
    })?;
    let legacy_bytes = serde_json::to_vec(&json!({"kind":"page","data":&first}))
        .unwrap()
        .len();
    let packed = LibraryResponse::Page(first.into());
    let packed_bytes = serde_json::to_vec(&packed).unwrap().len();
    metrics.insert(
        "packedPageEncode".into(),
        sample!(100, || {
            serde_json::to_vec(&packed).map_err(|e| {
                AppError::new(ErrorCode::Catalog, "Could not encode fixture page.").caused_by(e)
            })
        })?,
    );
    let mut result = json!({"sqlite":sqlite,"metrics":metrics,"boundaryLogicalPagerMissBytes":(after.0-before.0) as u64*before.2,
        "wire":{"legacyBytes":legacy_bytes,"packedBytes":packed_bytes,"rows":240,"format":1},
        "plans":plans,"trace":performance::snapshot(),"productionBackupAvailable":false});
    if segmented {
        result["traceSegments"] = json!(segments);
    }
    Ok(result)
}

pub fn measure_catalog(path: &Path, fixture: &Value) -> Result<Value> {
    performance::enable();
    let mut metrics = serde_json::Map::new();
    metrics.insert(
        "appReopenOsWarm".into(),
        measure(7, || Catalog::open(path.to_owned()).map(drop))?,
    );
    let catalog = Catalog::open(path.to_owned())?;
    let sqlite = sqlite_info(&catalog)?;
    let first = store::query(&catalog, AssetQuery::default(), None, 240)?;
    assert_eq!(first.total, fixture["rows"].as_u64().unwrap() as u32);
    metrics.insert(
        "firstPage".into(),
        measure(100, || {
            store::query(&catalog, AssetQuery::default(), None, 240)
        })?,
    );
    metrics.insert(
        "nextPage".into(),
        measure(100, || {
            store::query(
                &catalog,
                AssetQuery::default(),
                first.next_cursor.clone(),
                240,
            )
        })?,
    );
    let root = fixture["rootIds"][0].as_str().unwrap().to_owned();
    metrics.insert(
        "rootPage".into(),
        measure(50, || {
            store::query(
                &catalog,
                AssetQuery {
                    root_id: Some(root.clone()),
                    ..Default::default()
                },
                None,
                240,
            )
        })?,
    );
    metrics.insert(
        "tagFilter".into(),
        measure(30, || {
            store::query(
                &catalog,
                AssetQuery {
                    tag: "Fixture tag".into(),
                    ..Default::default()
                },
                None,
                240,
            )
        })?,
    );
    metrics.insert(
        "filenameFilter".into(),
        measure(30, || {
            store::query(
                &catalog,
                AssetQuery {
                    name: "000".into(),
                    ..Default::default()
                },
                None,
                240,
            )
        })?,
    );
    let mut seed = 20261028u64;
    metrics.insert(
        "randomDateSeek".into(),
        measure(100, || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let year = 1960 + (seed >> 32) % 65;
            store::seek(&catalog, AssetQuery::default(), year.to_string(), 240)
        })?,
    );
    metrics.insert(
        "timelineBuckets".into(),
        measure(10, || store::buckets(&catalog, AssetQuery::default()))?,
    );
    metrics.insert(
        "viewerNeighbors".into(),
        measure(100, || {
            store::neighbors(&catalog, AssetQuery::default(), first.assets[0].id.clone())
        })?,
    );
    metrics.insert(
        "freezeAll".into(),
        measure(3, || {
            let selection = store::freeze(&catalog, AssetQuery::default())?;
            assert_eq!(selection.count, first.total);
            // Release only the diagnostic selection just created in this owned fixture.
            catalog.call(move |db| {
                db.execute("DELETE FROM selections WHERE id=?", [selection.id])?;
                Ok(())
            })
        })?,
    );
    let payload = serde_json::to_vec(&first)
        .map_err(|e| {
            AppError::new(ErrorCode::WorkerFailed, "Diagnostic serialization failed.").caused_by(e)
        })?
        .len();
    let plans=catalog.call(|db|{
        let mut out=serde_json::Map::new();
        for (name,sql) in [
            ("page","SELECT * FROM assets WHERE is_media=1 AND presence<>'excluded' AND presence<>'missing' ORDER BY date DESC,id DESC LIMIT 240"),
            ("buckets","SELECT substr(date,1,7),count(*) FROM assets WHERE is_media=1 AND presence<>'excluded' AND presence<>'missing' GROUP BY substr(date,1,7)"),
            ("tagCount","SELECT count(*) FROM assets WHERE is_media=1 AND presence<>'excluded' AND presence<>'missing' AND EXISTS(SELECT 1 FROM json_each(tags) WHERE value='Fixture tag')")
        ] {let rows=db.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?.query_map([],|r|r.get::<_,String>(3))?.collect::<rusqlite::Result<Vec<_>>>()?;out.insert(name.into(),json!(rows));}
        Ok(out)
    })?;
    let trace = performance::snapshot();
    performance::disable();
    let bytes = std::fs::metadata(path)?.len();
    Ok(
        json!({"metrics":metrics,"sqlite":sqlite,"pageItems":first.assets.len(),"pageJsonBytes":payload,"catalogBytesAfterSelection":bytes,"queryPlans":plans,"trace":trace}),
    )
}

pub async fn measure_media(root: &Path, worker: &Path, working_directory: &Path) -> Result<Value> {
    let paths = workers::WorkerPaths {
        executable: worker.to_owned(),
        exiftool: None,
        working_directory: working_directory.to_owned(),
    };
    let mut results = serde_json::Map::new();
    performance::enable();
    for (label, relative) in [
        ("jpeg", "Coast/Generated-00.jpg"),
        ("png", "Woodland/Generated-01.png"),
        ("webp", "Woodland/Generated-05.webp"),
        ("unsupported", "unsupported.heic"),
        ("malformed", "broken.jpg"),
    ] {
        let relative = if label == "png" {
            "Woodland/Generated-01.png"
        } else {
            relative
        };
        let mut samples = Vec::new();
        let mut successes = 0;
        for _ in 0..10 {
            let start = Instant::now();
            let input = workers::read_input(filesystem::open_readonly(&root.join(relative))?)?;
            let result = workers::thumbnail(
                &paths,
                input,
                256,
                crate::media::PreviewFormat::Auto,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
            if result.is_ok() {
                successes += 1;
            }
            samples.push(elapsed(start));
        }
        results.insert(
            label.into(),
            json!({"timings":distribution(&samples),"successes":successes,"attempts":10}),
        );
    }
    results.insert(
        "inProcessJpegDecodeResizeEncode".into(),
        measure(30, || {
            crate::media::decode_thumbnail(&root.join("Coast/Generated-00.jpg"))
        })?,
    );
    Ok(json!({"metrics":results,"trace":performance::snapshot()}))
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    #[test]
    fn initial_previews_are_deferred_until_published_and_survive_restart() {
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("Fixture.stillharbour");
        let catalog = Catalog::open(path.clone()).unwrap();
        let fixture = seed_catalog(&catalog, owned.path(), 10_000).unwrap();
        let root = fixture["rootIds"][0].as_str().unwrap().to_owned();
        store::begin_job(&catalog, root.clone(), ScanFilter::default()).unwrap();
        let all = AssetQuery {
            root_id: Some(root.clone()),
            name: "Synthetic 0000001".into(),
            ..Default::default()
        };
        let mut wire = serde_json::to_value(&all).unwrap();
        wire["deferPendingPreviews"] = json!(true);
        let visual: AssetQuery = serde_json::from_value(wire).unwrap();
        let whole_root = AssetQuery {
            root_id: Some(root.clone()),
            defer_pending_previews: Some(true),
            ..Default::default()
        };
        assert_eq!(
            store::query(&catalog, whole_root.clone(), None, 240)
                .unwrap()
                .total,
            0,
            "Initial pending rows must not use the raw inventory counter"
        );
        let empty_selection = store::freeze(&catalog, whole_root.clone()).unwrap();
        assert_eq!(empty_selection.count, 0);
        assert!(
            store::query(&catalog, visual.clone(), None, 240)
                .unwrap()
                .assets
                .is_empty()
        );
        assert!(store::buckets(&catalog, visual.clone()).unwrap().is_empty());
        let original = store::query(&catalog, all, None, 240)
            .unwrap()
            .assets
            .remove(0);
        store::preview_state_for_edge(
            &catalog,
            ThumbnailKey {
                asset_id: original.id.clone(),
                revision: original.revision.clone(),
                edge: 32,
            },
            None,
        )
        .unwrap();
        let visible = store::query(&catalog, visual.clone(), None, 240).unwrap();
        assert_eq!(visible.total, 1);
        assert_eq!(visible.assets[0].id, original.id);
        assert_eq!(
            store::query(&catalog, whole_root.clone(), None, 240)
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            store::freeze(&catalog, whole_root.clone()).unwrap().count,
            1
        );
        let seek = store::seek_asset(
            &catalog,
            whole_root.clone(),
            original.id.clone(),
            AssetSeekPosition::At,
            240,
        )
        .unwrap();
        assert_eq!(seek.total, 1);
        assert_eq!(seek.assets[0].id, original.id);
        let neighbors = store::neighbors(&catalog, whole_root, original.id.clone()).unwrap();
        assert!(neighbors.next.is_none() && neighbors.previous.is_none());
        assert_eq!(
            store::buckets(&catalog, visual.clone())
                .unwrap()
                .iter()
                .map(|b| b.count)
                .sum::<u32>(),
            1
        );
        let initial = format!("thumbnail:initial:{root}");
        store::thumbnail_job_state(
            &catalog,
            initial.clone(),
            "canceled".into(),
            "Fixture cancellation".into(),
        )
        .unwrap();
        store::begin_job(&catalog, root.clone(), ScanFilter::default()).unwrap();
        assert_eq!(
            store::initial_preparation_job(&catalog, root.clone())
                .unwrap()
                .unwrap()
                .state,
            "canceled",
            "A reconciliation scan must not undo a paused preparation"
        );
        drop(catalog);
        let catalog = Catalog::open(path).unwrap();
        assert_eq!(
            serde_json::to_value(store::root(&catalog, root.clone()).unwrap().view).unwrap()["preparingPreviews"],
            true
        );
        assert_eq!(store::query(&catalog, visual, None, 240).unwrap().total, 1);
        assert_eq!(
            store::begin_thumbnail_job(&catalog, root.clone())
                .unwrap()
                .id,
            initial
        );
        store::thumbnail_job_state(
            &catalog,
            initial.clone(),
            "completed".into(),
            "Fixture completion attempt".into(),
        )
        .unwrap();
        assert_eq!(
            store::job(&catalog, initial.clone()).unwrap().state,
            "failed",
            "Incomplete preparation must not expose pending photos"
        );
        let failure=json!({"policy":crate::library::THUMBNAIL_POLICY,"message":"Fixture unavailable","edges":{"32":"Fixture unavailable"}}).to_string();
        catalog.call(move|db| {
            db.execute("UPDATE assets SET preview_state='unavailable',preview_error=? WHERE root_id=? AND preview_state='pending'",params![failure,root])?;
            Ok(())
        }).unwrap();
        store::thumbnail_job_state(
            &catalog,
            initial.clone(),
            "partial".into(),
            "Fixture previews resolved with failures".into(),
        )
        .unwrap();
        assert_eq!(store::job(&catalog, initial).unwrap().state, "partial");
    }

    fn assert_filename_query_parity(catalog: &Catalog, query: AssetQuery) {
        let (where_sql, values) = store::query_where(&query).unwrap();
        let order = match query.sort.as_str() {
            "name" => "name ASC,id ASC",
            "size" => "bytes DESC,id DESC",
            "modified" => "coalesce(modified_at,'') DESC,id DESC",
            "added" => "added_at DESC,id DESC",
            _ => "date DESC,id DESC",
        };
        let expected = catalog
            .call(move |db| {
                Ok(db
                    .prepare(&format!(
                        "SELECT {} FROM assets WHERE {where_sql} ORDER BY {order}",
                        store::ASSET_COLUMNS
                    ))?
                    .query_map(rusqlite::params_from_iter(values), store::asset_row)?
                    .collect::<rusqlite::Result<Vec<_>>>()?)
            })
            .unwrap();
        let mut actual = Vec::new();
        let mut cursor = None;
        loop {
            let page = store::query(catalog, query.clone(), cursor, 240).unwrap();
            assert_eq!(page.total as usize, expected.len());
            actual.extend(page.assets);
            assert!(actual.len() <= expected.len());
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        assert_eq!(
            serde_json::to_value(&actual).unwrap(),
            serde_json::to_value(&expected).unwrap()
        );
        assert_eq!(
            store::buckets(catalog, query.clone())
                .unwrap()
                .iter()
                .map(|b| b.count)
                .sum::<u32>() as usize,
            expected.len()
        );
        if !expected.is_empty() {
            let middle = expected.len() / 2;
            let id = expected[middle].id.clone();
            let at =
                store::seek_asset(catalog, query.clone(), id.clone(), AssetSeekPosition::At, 3)
                    .unwrap();
            assert_eq!(at.total as usize, expected.len());
            assert_eq!(
                serde_json::to_value(at.assets).unwrap(),
                serde_json::to_value(&expected[middle..expected.len().min(middle + 3)]).unwrap()
            );
            let before = store::seek_asset(
                catalog,
                query.clone(),
                id.clone(),
                AssetSeekPosition::Before,
                3,
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(before.assets).unwrap(),
                serde_json::to_value(&expected[middle.saturating_sub(3)..middle]).unwrap()
            );
            let neighbors = store::neighbors(catalog, query, id).unwrap();
            assert_eq!(
                neighbors.previous.map(|a| a.id),
                middle.checked_sub(1).map(|i| expected[i].id.clone())
            );
            assert_eq!(
                neighbors.next.map(|a| a.id),
                expected.get(middle + 1).map(|a| a.id.clone())
            );
        }
    }

    #[test]
    fn filename_scopes_preserve_thresholds_filters_cursors_and_native_keys() {
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("Fixture.stillharbour");
        let catalog = Catalog::open(path.clone()).unwrap();
        let fixture = seed_catalog(&catalog, owned.path(), 10_000).unwrap();
        catalog
            .call(|db| {
                db.execute(
                    "UPDATE assets SET name='Threshold' WHERE CAST(native_file AS INTEGER)<=1024",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        for count in [1024, 1025] {
            if count == 1025 {
                // No revision bump: the cache and candidate scope must both refresh.
                let external = rusqlite::Connection::open(&path).unwrap();
                external
                    .execute(
                        "UPDATE assets SET name='Threshold' WHERE native_file='1025'",
                        [],
                    )
                    .unwrap();
            }
            for sort in ["capture", "name", "size", "modified", "added"] {
                assert_filename_query_parity(
                    &catalog,
                    AssetQuery {
                        name: "THRESHOLD".into(),
                        sort: sort.into(),
                        ..Default::default()
                    },
                );
            }
            assert_eq!(
                store::query(
                    &catalog,
                    AssetQuery {
                        name: "Threshold".into(),
                        ..Default::default()
                    },
                    None,
                    1
                )
                .unwrap()
                .total,
                count
            );
            if count == 1025 {
                // A small visible total does not bound excluded name matches.
                // The complete lookahead must fall back rather than drop id1025.
                catalog
                    .call(|db| {
                        db.execute(
                            "UPDATE assets SET presence='excluded' WHERE native_file='1'",
                            [],
                        )?;
                        Ok(())
                    })
                    .unwrap();
                assert_filename_query_parity(
                    &catalog,
                    AssetQuery {
                        name: "Threshold".into(),
                        ..Default::default()
                    },
                );
                catalog
                    .call(|db| {
                        db.execute(
                            "UPDATE assets SET presence='present' WHERE native_file='1'",
                            [],
                        )?;
                        Ok(())
                    })
                    .unwrap();
            }
        }
        catalog
            .call(|db| {
                db.execute(
                    "UPDATE assets SET name=?,favorite=1,rowid=? WHERE native_file='1'",
                    params!["A%_\\É'Photo", i64::MAX - 7],
                )?;
                db.execute(
                    "UPDATE assets SET presence='missing' WHERE native_file='2'",
                    [],
                )?;
                db.execute(
                    "UPDATE assets SET presence='excluded' WHERE native_file='3'",
                    [],
                )?;
                db.execute("UPDATE assets SET is_media=0 WHERE native_file='4'", [])?;
                db.execute(
                    "UPDATE assets SET folder='Trip',folder_native=? WHERE native_file='5'",
                    [b"Trip".as_slice()],
                )?;
                Ok(())
            })
            .unwrap();
        let root = fixture["rootIds"][1].as_str().unwrap().to_owned();
        for query in [
            AssetQuery {
                name: "Threshold".into(),
                ..Default::default()
            },
            AssetQuery {
                name: "Threshold".into(),
                include_missing: true,
                ..Default::default()
            },
            AssetQuery {
                name: "Threshold".into(),
                root_id: Some(root.clone()),
                tag: "Fixture tag".into(),
                ..Default::default()
            },
            AssetQuery {
                name: "Threshold".into(),
                root_id: Some(root.clone()),
                directory_id: Some(super::super::directory::encode_id(b"")),
                ..Default::default()
            },
            AssetQuery {
                name: "Threshold".into(),
                root_id: Some(root),
                folder: "Trip".into(),
                ..Default::default()
            },
            AssetQuery {
                name: "Synthetic 0002".into(),
                extension: "jpg".into(),
                from_date: "1960-01-01".into(),
                through_date: "2024-12-31".into(),
                min_bytes: "2000".into(),
                max_bytes: "10000".into(),
                sort: "size".into(),
                ..Default::default()
            },
            AssetQuery {
                name: "%_\\É'".into(),
                favorites_only: true,
                ..Default::default()
            },
            AssetQuery {
                name: "'%' OR 1=1--".into(),
                ..Default::default()
            },
        ] {
            assert_filename_query_parity(&catalog, query);
        }
    }

    #[test]
    fn cold_filename_counts_skip_unmatched_row_fields() {
        let owned = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(owned.path().join("Fixture.stillharbour")).unwrap();
        seed_catalog(&catalog, owned.path(), 10_000).unwrap();
        for name in ["Generated", "Synthetic 0000", "Synthetic 000999"] {
            let query = AssetQuery {
                name: name.into(),
                ..Default::default()
            };
            let (predicate, values) = store::query_where(&query).unwrap();
            catalog.call(move |db| {
                let mut actual = db.prepare(&format!("SELECT count(*) FROM assets WHERE {predicate}"))?;
                let count: u32 = actual.query_row(rusqlite::params_from_iter(&values), |r|r.get(0))?;
                let actual_steps = actual.get_status(rusqlite::StatementStatus::VmStep);
                // The reference retains media rejection, then skips presence
                // checks on filename nonmatches in each wide asset record.
                let mut reference = db.prepare("SELECT count(*) FROM assets WHERE is_media=1 AND name LIKE ? ESCAPE '\\' AND presence NOT IN ('excluded','missing')")?;
                let expected: u32 = reference.query_row([format!("%{}%",store::escape_like(&query.name))], |r|r.get(0))?;
                let reference_steps = reference.get_status(rusqlite::StatementStatus::VmStep);
                assert_eq!(count, expected);
                eprintln!("Cold filename {}: {actual_steps} VM steps; reference {reference_steps}", query.name);
                assert!(actual_steps <= reference_steps + 64, "Cold filename count evaluates unmatched row fields: {actual_steps} vs {reference_steps}");
                Ok(())
            }).unwrap();
        }
    }

    #[test]
    fn selective_filename_projection_avoids_full_asset_scans() {
        let owned = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(owned.path().join("Fixture.stillharbour")).unwrap();
        seed_catalog(&catalog, owned.path(), 10_000).unwrap();
        let query = AssetQuery {
            name: "Synthetic 0000".into(),
            ..Default::default()
        };
        // Isolate the ordered projection after the exact count is available;
        // cold counts deliberately retain the ordinary SQL scan.
        assert_eq!(
            store::query(&catalog, query.clone(), None, 240)
                .unwrap()
                .total,
            999
        );
        catalog
            .call(|db| {
                db.execute_batch("PRAGMA shrink_memory")?;
                Ok(())
            })
            .unwrap();
        let before = catalog
            .call(|db| Ok(performance::pager_counters(db).unwrap()))
            .unwrap();
        let page = store::query(&catalog, query, None, 240).unwrap();
        let after = catalog
            .call(|db| Ok(performance::pager_counters(db).unwrap()))
            .unwrap();
        assert_eq!(page.total, 999);
        assert_eq!(page.assets.len(), 240);
        assert!(page.next_cursor.is_some());
        let bytes = (after.0 - before.0) as u64 * before.2;
        eprintln!("Selective filename projection logical pager bytes: {bytes}");
        assert!(
            bytes < 2 * 1024 * 1024,
            "Selective filename page read {bytes} logical pager bytes"
        );
    }

    #[test]
    fn positive_timeline_supplies_exact_page_and_seek_counts() {
        let owned = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(owned.path().join("Fixture.stillharbour")).unwrap();
        seed_catalog(&catalog, owned.path(), 10_000).unwrap();
        for sort in ["capture", "name", "size", "modified", "added"] {
            let query = AssetQuery {
                name: "Synthetic".into(),
                sort: sort.into(),
                ..Default::default()
            };
            let buckets = store::buckets(&catalog, query.clone()).unwrap();
            assert_eq!(buckets.iter().map(|b| b.count).sum::<u32>(), 10_000);
            catalog
                .call(|db| {
                    db.execute_batch("PRAGMA shrink_memory")?;
                    Ok(())
                })
                .unwrap();
            let before = catalog
                .call(|db| Ok(performance::pager_counters(db).unwrap()))
                .unwrap();
            let page = store::query(&catalog, query.clone(), None, 1).unwrap();
            let seek = store::seek_asset(
                &catalog,
                query.clone(),
                page.assets[0].id.clone(),
                AssetSeekPosition::At,
                1,
            )
            .unwrap();
            assert_eq!(page.total, 10_000);
            assert_eq!(seek.total, 10_000);
            assert_eq!(seek.assets[0].id, page.assets[0].id);
            let after = catalog
                .call(|db| Ok(performance::pager_counters(db).unwrap()))
                .unwrap();
            let bytes = (after.0 - before.0) as u64 * before.2;
            // Measure the identical ordered projection with its count already
            // cached. Selective sorts can still scan; isolate the redundant count.
            catalog
                .call(|db| {
                    db.execute_batch("PRAGMA shrink_memory")?;
                    Ok(())
                })
                .unwrap();
            let before = catalog
                .call(|db| Ok(performance::pager_counters(db).unwrap()))
                .unwrap();
            let reference = store::query(&catalog, query.clone(), None, 1).unwrap();
            store::seek_asset(
                &catalog,
                query,
                reference.assets[0].id.clone(),
                AssetSeekPosition::At,
                1,
            )
            .unwrap();
            let after = catalog
                .call(|db| Ok(performance::pager_counters(db).unwrap()))
                .unwrap();
            let reference_bytes = (after.0 - before.0) as u64 * before.2;
            assert_eq!(
                serde_json::to_value(&page).unwrap(),
                serde_json::to_value(&reference).unwrap()
            );
            eprintln!(
                "Positive timeline/{sort} page+seek logical pager bytes: {bytes}; cached-count reference: {reference_bytes}"
            );
            assert!(
                bytes <= reference_bytes + 64 * 1024,
                "Positive timeline/{sort} repeated {bytes} logical pager bytes"
            );
        }
    }

    #[test]
    fn exact_empty_pages_do_not_repeat_the_asset_scan() {
        let owned = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(owned.path().join("Fixture.stillharbour")).unwrap();
        seed_catalog(&catalog, owned.path(), 10_000).unwrap();
        for sort in ["capture", "name", "size", "modified", "added"] {
            let query = AssetQuery {
                name: "Generated".into(),
                sort: sort.into(),
                ..Default::default()
            };
            assert_eq!(
                store::query(&catalog, query.clone(), None, 240)
                    .unwrap()
                    .total,
                0
            );
            catalog
                .call(|db| {
                    db.execute_batch("PRAGMA shrink_memory")?;
                    Ok(())
                })
                .unwrap();
            let before = catalog
                .call(|db| Ok(performance::pager_counters(db).unwrap()))
                .unwrap();
            let page = store::query(&catalog, query, None, 240).unwrap();
            let after = catalog
                .call(|db| Ok(performance::pager_counters(db).unwrap()))
                .unwrap();
            assert!(page.assets.is_empty() && page.next_cursor.is_none() && page.total == 0);
            let bytes = (after.0 - before.0) as u64 * before.2;
            eprintln!("Empty {sort} page logical pager bytes: {bytes}");
            assert!(
                bytes < 64 * 1024,
                "Empty {sort} page repeated {bytes} logical pager bytes"
            );
        }
        // Streaming scans may publish rows before refreshing inventory counters.
        catalog
            .call(|db| {
                db.execute("UPDATE roots SET file_count=0", [])?;
                Ok(())
            })
            .unwrap();
        assert_eq!(
            store::buckets(&catalog, AssetQuery::default())
                .unwrap()
                .iter()
                .map(|b| b.count)
                .sum::<u32>(),
            10_000
        );
        let partial = store::query(&catalog, AssetQuery::default(), None, 240).unwrap();
        assert_eq!(partial.total, 0);
        assert_eq!(partial.assets.len(), 240);
        let exact = AssetQuery {
            include_missing: true,
            ..Default::default()
        };
        assert_eq!(
            store::buckets(&catalog, exact.clone())
                .unwrap()
                .iter()
                .map(|b| b.count)
                .sum::<u32>(),
            10_000
        );
        assert_eq!(
            store::query(&catalog, exact, None, 240).unwrap().total,
            10_000
        );
    }

    #[test]
    fn empty_pages_and_timeline_share_exact_counts_without_stale_results() {
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("Fixture.stillharbour");
        let catalog = Catalog::open(path.clone()).unwrap();
        seed_catalog(&catalog, owned.path(), 10_000).unwrap();
        let query = AssetQuery {
            name: "Generated".into(),
            ..Default::default()
        };
        assert!(store::buckets(&catalog, query.clone()).unwrap().is_empty());
        catalog
            .call(|db| {
                db.execute_batch("PRAGMA shrink_memory")?;
                Ok(())
            })
            .unwrap();
        let before = catalog
            .call(|db| Ok(performance::pager_counters(db).unwrap()))
            .unwrap();
        let page = store::query(&catalog, query.clone(), None, 240).unwrap();
        let after = catalog
            .call(|db| Ok(performance::pager_counters(db).unwrap()))
            .unwrap();
        let bytes = (after.0 - before.0) as u64 * before.2;
        assert_eq!(page.total, 0);
        eprintln!("Empty timeline/page logical pager bytes: {bytes}");
        assert!(
            bytes < 64 * 1024,
            "Empty timeline/page repeated {bytes} logical pager bytes"
        );
        assert_eq!(
            store::query(&catalog, query.clone(), Some("invalid".into()), 240)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            store::query(&catalog, query.clone(), None, 0)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        for invalid in [
            AssetQuery {
                min_bytes: "-1".into(),
                ..query.clone()
            },
            AssetQuery {
                sort: "unknown".into(),
                ..query.clone()
            },
        ] {
            assert_eq!(
                store::query(&catalog, invalid, None, 240).unwrap_err().code,
                ErrorCode::InvalidInput
            );
        }
        catalog.call(|db| {
            db.execute("UPDATE assets SET name='Generated new' WHERE id='70000000-0000-4000-8000-000000000001'",[])?;
            Ok(())
        }).unwrap();
        assert_eq!(
            store::query(&catalog, query.clone(), None, 240)
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            store::buckets(&catalog, query.clone())
                .unwrap()
                .iter()
                .map(|b| b.count)
                .sum::<u32>(),
            1
        );
        let external = rusqlite::Connection::open(path).unwrap();
        external.execute("UPDATE assets SET name='Synthetic restored' WHERE id='70000000-0000-4000-8000-000000000001'",[]).unwrap();
        assert_eq!(
            store::query(&catalog, query.clone(), None, 240)
                .unwrap()
                .total,
            0
        );
        catalog
            .call(|db| {
                db.execute_batch("PRAGMA shrink_memory")?;
                Ok(())
            })
            .unwrap();
        let before = catalog
            .call(|db| Ok(performance::pager_counters(db).unwrap()))
            .unwrap();
        assert!(store::buckets(&catalog, query).unwrap().is_empty());
        let after = catalog
            .call(|db| Ok(performance::pager_counters(db).unwrap()))
            .unwrap();
        let bytes = (after.0 - before.0) as u64 * before.2;
        eprintln!("Empty page/timeline logical pager bytes: {bytes}");
        assert!(
            bytes < 64 * 1024,
            "Empty page/timeline repeated {bytes} logical pager bytes"
        );
    }

    #[test]
    fn summary_cache_observes_actor_and_external_writes_and_query_scope() {
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("Fixture.stillharbour");
        let catalog = Catalog::open(path.clone()).unwrap();
        let fixture = seed_catalog(&catalog, owned.path(), 10_000).unwrap();
        let tagged = AssetQuery {
            tag: "Fixture tag".into(),
            ..Default::default()
        };
        let count = |q: AssetQuery| store::query(&catalog, q, None, 1).unwrap().total;
        let bucket_count = |q| {
            store::buckets(&catalog, q)
                .unwrap()
                .iter()
                .map(|b| b.count)
                .sum::<u32>()
        };
        assert_eq!(bucket_count(tagged.clone()), 3333);
        assert_eq!(count(tagged.clone()), 3333);
        assert_eq!(bucket_count(AssetQuery::default()), 10_000);
        let root_query = AssetQuery {
            root_id: Some(fixture["rootIds"][1].as_str().unwrap().into()),
            ..Default::default()
        };
        assert_eq!(bucket_count(root_query.clone()), 2000);
        assert_eq!(count(root_query.clone()), 2000);
        let root_tagged = AssetQuery {
            tag: tagged.tag.clone(),
            ..root_query
        };
        assert_eq!(bucket_count(root_tagged.clone()), 666);
        assert_eq!(count(root_tagged), 666);
        // No library_state bump: cache freshness cannot rely on callers doing it.
        catalog
            .call(|db| {
                db.execute(
                    "UPDATE assets SET tags='[]' WHERE id='70000000-0000-4000-8000-000000000003'",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        assert_eq!(bucket_count(tagged.clone()), 3332);
        assert_eq!(count(tagged.clone()), 3332);
        let external = rusqlite::Connection::open(path).unwrap();
        external.execute("UPDATE assets SET presence='missing' WHERE id='70000000-0000-4000-8000-000000000006'",[]).unwrap();
        assert_eq!(count(tagged.clone()), 3331);
        assert_eq!(bucket_count(tagged.clone()), 3331);
        assert_eq!(
            count(AssetQuery {
                include_missing: true,
                ..tagged
            }),
            3332
        );
        assert_eq!(count(AssetQuery::default()), 9999);
        let unknown_before = store::buckets(&catalog, AssetQuery::default())
            .unwrap()
            .iter()
            .find(|b| b.year == "Undated")
            .unwrap()
            .count;
        catalog
            .call(|db| {
                db.execute(
                    "UPDATE assets SET date='' WHERE id='70000000-0000-4000-8000-000000000009'",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        let buckets = store::buckets(&catalog, AssetQuery::default()).unwrap();
        assert_eq!(
            buckets.iter().find(|b| b.year == "Undated").unwrap().count,
            unknown_before + 1
        );
        let added = store::buckets(
            &catalog,
            AssetQuery {
                sort: "added".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].year, "2026");
    }

    #[test]
    fn indexed_pages_and_neighbors_preserve_ties_unknown_dates_and_large_sizes() {
        let owned = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(owned.path().join("Fixture.stillharbour")).unwrap();
        seed_catalog(&catalog, owned.path(), 10_000).unwrap();
        let all = catalog
            .call(|db| {
                Ok(db
                    .prepare(&format!("SELECT {} FROM assets", store::ASSET_COLUMNS))?
                    .query_map([], store::asset_row)?
                    .collect::<rusqlite::Result<Vec<_>>>()?)
            })
            .unwrap();
        for sort in ["capture", "name", "size", "added", "modified"] {
            let mut expected = all.clone();
            expected.sort_by(|a, b| {
                let order = match sort {
                    "name" => a.name.cmp(&b.name).then(a.id.cmp(&b.id)),
                    "size" => a
                        .bytes
                        .parse::<u64>()
                        .unwrap()
                        .cmp(&b.bytes.parse().unwrap())
                        .then(a.id.cmp(&b.id)),
                    "added" => a.added_at.cmp(&b.added_at).then(a.id.cmp(&b.id)),
                    "modified" => a.modified_at.cmp(&b.modified_at).then(a.id.cmp(&b.id)),
                    _ => a.date.cmp(&b.date).then(a.id.cmp(&b.id)),
                };
                if sort == "name" {
                    order
                } else {
                    order.reverse()
                }
            });
            let query = AssetQuery {
                sort: sort.into(),
                ..Default::default()
            };
            let mut actual = Vec::new();
            let mut cursor = None;
            loop {
                let page = store::query(&catalog, query.clone(), cursor, 240).unwrap();
                actual.extend(page.assets.into_iter().map(|a| a.id));
                cursor = page.next_cursor;
                if cursor.is_none() {
                    break;
                }
            }
            assert_eq!(
                actual,
                expected.iter().map(|a| a.id.clone()).collect::<Vec<_>>(),
                "{sort}"
            );
            for index in [0, 4999, 9999] {
                let neighbors =
                    store::neighbors(&catalog, query.clone(), expected[index].id.clone()).unwrap();
                assert_eq!(
                    neighbors.previous.map(|a| a.id),
                    index.checked_sub(1).map(|i| expected[i].id.clone()),
                    "{sort}"
                );
                assert_eq!(
                    neighbors.next.map(|a| a.id),
                    expected.get(index + 1).map(|a| a.id.clone()),
                    "{sort}"
                );
            }
        }
    }
    #[test]
    fn boundary_neighbors_use_a_bounded_index_seek() {
        let owned = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(owned.path().join("Fixture.stillharbour")).unwrap();
        seed_catalog(&catalog, owned.path(), 10_000).unwrap();
        let first = store::query(&catalog, AssetQuery::default(), None, 1).unwrap();
        // Empty SQLite's connection-local pager cache, without purging OS caches.
        catalog
            .call(|db| {
                db.execute_batch("PRAGMA shrink_memory")?;
                Ok(())
            })
            .unwrap();
        let before = catalog
            .call(|db| Ok(performance::pager_counters(db).unwrap()))
            .unwrap();
        let neighbors =
            store::neighbors(&catalog, AssetQuery::default(), first.assets[0].id.clone()).unwrap();
        let after = catalog
            .call(|db| Ok(performance::pager_counters(db).unwrap()))
            .unwrap();
        assert!(neighbors.previous.is_none());
        assert!(neighbors.next.is_some());
        let missed_bytes = (after.0 - before.0) as u64 * before.2;
        eprintln!("Boundary seek logical pager bytes: {missed_bytes}");
        assert!(
            missed_bytes < 512 * 1024,
            "Boundary seek read {missed_bytes} logical pager bytes"
        );
    }
    #[test]
    fn seeded_observations_cover_the_contract_without_media_decodes() {
        let owned = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(owned.path().join("Fixture.stillharbour")).unwrap();
        assert!(seed_catalog(&catalog, owned.path(), 9999).is_err());
        seed_catalog(&catalog, owned.path(), 10_000).unwrap();
        let coverage = catalog.call(|db| {
            Ok(db.query_row("SELECT count(*),sum(date=''),sum(width=12000 AND height=40),sum(width=40 AND height=12000),sum(extension IN ('mp4','mov')),max(bytes),sum(json_array_length(tags)>0) FROM assets", [], |r| Ok((r.get::<_,u32>(0)?,r.get::<_,u32>(1)?,r.get::<_,u32>(2)?,r.get::<_,u32>(3)?,r.get::<_,u32>(4)?,r.get::<_,i64>(5)?,r.get::<_,u32>(6)?)))?)
        }).unwrap();
        assert_eq!(coverage.0, 10_000);
        assert_eq!(coverage.1, 500);
        assert!(coverage.2 > 1000 && coverage.3 > 1000);
        assert_eq!(coverage.4, 2000);
        assert_eq!(coverage.5, 8589934592);
        assert!(coverage.6 > 3000);
        let offline = catalog
            .call(|db| {
                Ok(db.query_row(
                    "SELECT count(*) FROM roots WHERE availability='offline'",
                    [],
                    |r| r.get::<_, u32>(0),
                )?)
            })
            .unwrap();
        assert_eq!(offline, 2);
        assert_eq!(
            store::query(&catalog, AssetQuery::default(), None, 240)
                .unwrap()
                .assets
                .len(),
            240
        );
    }
}
