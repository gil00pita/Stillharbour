//! Owned synthetic catalog observations, never a photo-library benchmark claim.
use super::{model::*, native, store};
use crate::{catalog::Catalog, domain::Result, filesystem};
use serde::Serialize;
use std::time::Instant;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    rows: u32,
    seed_ms: f64,
    first_query_ms: f64,
    warm_query_p50_ms: f64,
    warm_query_p95_ms: f64,
    year_seek_p50_ms: f64,
    year_seek_p95_ms: f64,
    buckets_ms: f64,
    frozen_selection_ms: f64,
    selected: u32,
    max_rss_bytes: i64,
    database_bytes: u64,
    seek_plan: Vec<String>,
}
fn ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
fn percentile(mut samples: Vec<f64>, fraction: f64) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[((samples.len() - 1) as f64 * fraction).ceil() as usize]
}
pub fn run(rows: u32) -> Result<Observation> {
    assert!(matches!(rows, 100_000 | 1_000_000));
    let owned = tempfile::Builder::new()
        .prefix("stillharbour-synthetic-catalog-")
        .tempdir()?;
    let source = owned.path().join("empty-generated-root");
    std::fs::create_dir(&source)?;
    let path = owned.path().join("synthetic.stillharbour");
    let catalog = Catalog::open(path.clone())?;
    let handle = native::open_root(&source, None)?;
    let root = store::add_root(
        &catalog,
        source,
        filesystem::identity(&handle)?,
        native::volume_token(&handle)?,
        "Synthetic catalog only".into(),
        RootMode::Referenced,
        ScanFilter::default(),
    )?;
    let start = Instant::now();
    catalog.call(move|db|{
        let tx=db.transaction()?;
        tx.execute("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<?1) INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,modified_at,born_at,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan) SELECT printf('00000000-0000-4000-8000-%012d',x),?2,CAST(printf('synthetic-%07d.jpg',x) AS BLOB),printf('synthetic-%07d.jpg',x),X'','','Synthetic-'||x||'.jpg','Synthetic-'||x,'jpg',CAST(x AS TEXT),1024+x,printf('%064d',x),'2026-01-01T00:00:00Z',NULL,'2026-01-01T00:00:00Z',printf('%04d-%02d-%02dT12:00:00',1960+x%65,1+x%12,1+x%28),'generated_capture','ready','pending','present',1,'synthetic-only' FROM n",rusqlite::params![rows,root])?;
        tx.commit()?; db.execute_batch("ANALYZE; PRAGMA wal_checkpoint(TRUNCATE);")?; Ok(())
    })?;
    let seed_ms = ms(start);
    let start = Instant::now();
    let first = store::query(&catalog, AssetQuery::default(), None, 240)?;
    let first_query_ms = ms(start);
    assert_eq!(first.total, rows);
    let mut query_times = Vec::new();
    let mut seeks = Vec::new();
    for _ in 0..20 {
        let start = Instant::now();
        let page = store::query(
            &catalog,
            AssetQuery::default(),
            first.next_cursor.clone(),
            240,
        )?;
        query_times.push(ms(start));
        assert_eq!(page.assets.len(), 240);
        let start = Instant::now();
        let page = store::seek(&catalog, AssetQuery::default(), "1964".into(), 240)?;
        seeks.push(ms(start));
        assert!(page.assets[0].date.starts_with("1964"));
    }
    let start = Instant::now();
    let buckets = store::buckets(&catalog, AssetQuery::default())?;
    let buckets_ms = ms(start);
    assert_eq!(buckets.iter().map(|b| b.count).sum::<u32>(), rows);
    let start = Instant::now();
    let selection = store::freeze(&catalog, AssetQuery::default())?;
    let frozen_selection_ms = ms(start);
    let seek_plan=catalog.call(|db|Ok(db.prepare("EXPLAIN QUERY PLAN SELECT * FROM assets WHERE is_media=1 AND presence<>'excluded' AND presence<>'missing' AND (date,id)<('1964-12-31T99','~') ORDER BY date DESC,id DESC LIMIT 240")?.query_map([],|row|row.get(3))?.collect::<rusqlite::Result<Vec<String>>>()?))?;
    let max_rss_bytes = {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        // SAFETY: getrusage initializes the correctly sized output before it is read.
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } == 0 {
            unsafe { usage.assume_init() }.ru_maxrss
        } else {
            0
        }
    };
    let database_bytes = std::fs::metadata(&path)?.len();
    Ok(Observation {
        rows,
        seed_ms,
        first_query_ms,
        warm_query_p50_ms: percentile(query_times.clone(), 0.5),
        warm_query_p95_ms: percentile(query_times, 0.95),
        year_seek_p50_ms: percentile(seeks.clone(), 0.5),
        year_seek_p95_ms: percentile(seeks, 0.95),
        buckets_ms,
        frozen_selection_ms,
        selected: selection.count,
        max_rss_bytes,
        database_bytes,
        seek_plan,
    })
}
