//! Explicit synthetic read contenders; not a backup or media decode benchmark.
use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    sync::{Arc, Barrier, atomic::AtomicBool},
    time::{Duration, Instant},
};
use stillharbour_core::{
    filesystem,
    jobs::resources::{Class, ResourceScheduler, Work},
    performance,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--run" {
        return Err("Use --run UUID; only generated .context/scheduling fixtures accepted".into());
    }
    let run = uuid::Uuid::parse_str(&args[1])?.to_string();
    let namespace = PathBuf::from(".context/scheduling");
    let owned = namespace.join(&run);
    for path in [
        PathBuf::from(".context"),
        namespace,
        owned.clone(),
        owned.join("sources"),
    ] {
        let info = std::fs::symlink_metadata(path)?;
        if !info.is_dir() || info.file_type().is_symlink() {
            return Err("Fixture namespace must be an owned ordinary directory".into());
        }
    }
    let owner: serde_json::Value =
        serde_json::from_reader(File::open(owned.join("ownership.json"))?)?;
    if owner["run"] != run || owner["kind"] != "p3-synthetic-read-contention" {
        return Err("Fixture ownership mismatch".into());
    }
    performance::enable();
    let scheduler = ResourceScheduler::default();
    let barrier = Arc::new(Barrier::new(4));
    let mut workers = Vec::new();
    for (index, (name, class, work)) in [
        ("visible_read_fixture", Class::Interactive, Work::Rendition),
        ("metadata_read_fixture", Class::Background, Work::Metadata),
        ("backup_read_fixture", Class::Background, Work::Metadata),
        (
            "thumbnail_prebuild_fixture",
            Class::Background,
            Work::Rendition,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let scheduler = scheduler.clone();
        let barrier = barrier.clone();
        let source = owned.join("sources").join(format!("{index}.bin"));
        workers.push(std::thread::spawn(move || -> Result<_, String> {
            let mut wait_us = Vec::new();
            let mut step_us = Vec::new();
            let mut bytes = 0u64;
            let canceled = AtomicBool::new(false);
            barrier.wait();
            for _ in 0..60 {
                let mut file = filesystem::open_readonly(&source).map_err(|e| e.to_string())?;
                let identity = filesystem::identity(&file).map_err(|e| e.to_string())?;
                let start = Instant::now();
                let _lease = scheduler.acquire(identity.volume, class, work, &canceled).map_err(|e| e.to_string())?;
                wait_us.push(start.elapsed().as_micros() as u64);
                let step = Instant::now();
                let mut buffer = [0u8; 65536];
                loop {
                    let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
                    if n == 0 { break; }
                    bytes += n as u64;
                }
                // Known simulated bounded helper step, not decoder/backup evidence.
                std::thread::sleep(Duration::from_millis(2));
                step_us.push(step.elapsed().as_micros() as u64);
            }
            Ok(serde_json::json!({"class":name,"waitUs":wait_us,"stepUs":step_us,"logicalReadBytes":bytes}))
        }));
    }
    let mut results = Vec::new();
    for worker in workers {
        results.push(worker.join().map_err(|_| "Fixture worker panicked")??);
    }
    println!(
        "{}",
        serde_json::json!({
            "version":1,"run":run,"kind":"synthetic_read_contention", "sourceWrites":0,
            "topology":"one actual source volume; four generated files", "results":results,
            "trace":performance::snapshot(),
        })
    );
    Ok(())
}
