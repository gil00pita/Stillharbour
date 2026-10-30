//! Revisit only a previously generated performance catalog; accepts no library path.
use std::{fs::File, io::Read, path::PathBuf};
use stillharbour_core::library::performance_probe;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 4 || args[0] != "--run" || args[2] != "--rows" {
        return Err("Use --run UUID --rows 10000|100000|1000000 for an owned fixture".into());
    }
    let run = uuid::Uuid::parse_str(&args[1])?.to_string();
    let rows: u32 = args[3].parse()?;
    if !matches!(rows, 10_000 | 100_000 | 1_000_000) {
        return Err("Unsupported fixture size".into());
    }
    let namespace = PathBuf::from(".context/performance");
    let owned = namespace.join(format!("{run}-{rows}"));
    for path in [
        PathBuf::from(".context"),
        namespace,
        owned.clone(),
        owned.join("sources"),
    ] {
        let info = std::fs::symlink_metadata(path)?;
        if !info.is_dir() || info.file_type().is_symlink() {
            return Err("Ordinary fixture directories required".into());
        }
    }
    for name in ["ownership.json", "Fixture.stillharbour"] {
        let info = std::fs::symlink_metadata(owned.join(name))?;
        if !info.is_file() || info.file_type().is_symlink() {
            return Err("Ordinary fixture files required".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if info.nlink() != 1 {
                return Err("Hard-linked fixture file rejected".into());
            }
        }
    }
    let marker = owned.join("ownership.json");
    let mut bytes = Vec::new();
    File::open(marker)?.take(4097).read_to_end(&mut bytes)?;
    if bytes.len() > 4096 {
        return Err("Oversized fixture marker".into());
    }
    let owner: serde_json::Value = serde_json::from_slice(&bytes)?;
    if owner["run"] != run
        || owner["rows"] != rows
        || owner["synthetic"] != true
        || owner["version"] != 1
    {
        return Err("Fixture ownership mismatch".into());
    }
    let result = performance_probe::measure_filtered(&owned.join("Fixture.stillharbour"), rows)?;
    println!(
        "{}",
        serde_json::json!({"version":1,"phase":"P4-filtered","run":run,"rows":rows,
        "profile":if cfg!(debug_assertions){"debug"}else{"release"},"results":result})
    );
    Ok(())
}
