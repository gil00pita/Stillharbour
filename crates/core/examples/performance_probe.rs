#[cfg(target_os = "macos")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{
        io::{BufRead, Read, Write},
        path::PathBuf,
        time::Instant,
    };
    use stillharbour_core::{
        catalog::Catalog, library::performance_probe, library_fixture::LibraryFixture,
    };
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let p1 = args.len() == 5 && args[4] == "--p1";
    let p4 = args.len() == 5 && args[4] == "--p4";
    if (args.len() != 4 && !p1 && !p4) || args[0] != "--rows" || args[2] != "--run" {
        return Err("Use --rows 10000|100000|1000000 --run UUID [--p1|--p4]".into());
    }
    let rows = args[1].parse::<u32>()?;
    if !matches!(rows, 10_000 | 100_000 | 1_000_000) {
        return Err("Unsupported fixture size".into());
    }
    let run = uuid::Uuid::parse_str(&args[3])?.to_string();
    let base = PathBuf::from(".context/performance");
    std::fs::create_dir_all(&base)?;
    let owned = base.join(format!("{run}-{rows}"));
    std::fs::create_dir(&owned)?;
    std::fs::write(
        owned.join("ownership.json"),
        serde_json::to_vec(
            &serde_json::json!({"version":1,"run":run,"rows":rows,"synthetic":true}),
        )?,
    )?;
    let sources = owned.join("sources");
    std::fs::create_dir(&sources)?;
    let fixture = LibraryFixture::generate_in(Some(&sources))?;
    let source = fixture.retain(); // Retain fixtures and before/after evidence; no automatic cleanup.
    let path = owned.join("Fixture.stillharbour");
    let started = Instant::now();
    let catalog = Catalog::open(path.clone())?;
    let fixture = performance_probe::seed_catalog(&catalog, &owned, rows)?;
    drop(catalog);
    let seed_ms = started.elapsed().as_secs_f64() * 1000.0;
    println!(
        "{}",
        serde_json::json!({"ready":true,"rows":rows,"run":run,"seedMs":seed_ms})
    );
    std::io::stdout().flush()?;
    let mut command = String::new();
    std::io::stdin().lock().take(64).read_line(&mut command)?;
    if command.trim() != "measure" {
        return Err("Independent before-manifest handshake required".into());
    }
    let results = if p4 {
        performance_probe::measure_p4(&path, &fixture)?
    } else if p1 {
        performance_probe::measure_p1(&path, &fixture)?
    } else {
        performance_probe::measure_catalog(&path, &fixture)?
    };
    let media = performance_probe::measure_media(
        &source,
        &PathBuf::from("target/release/stillharbour").canonicalize()?,
        &owned,
    )
    .await?;
    println!(
        "{}",
        serde_json::json!({"run":run,"rows":rows,"phase":if p4 {"P4"} else if p1 {"P1"} else {"P0"},"fixture":fixture,"profile":if cfg!(debug_assertions){"debug"}else{"release"},"seedMs":seed_ms,"catalog":results,"media":media})
    );
    Ok(())
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("P0 native fixture adapter is currently macOS-only; no Windows evidence.");
    std::process::exit(2);
}
