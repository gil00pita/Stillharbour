use stillharbour_core::{
    catalog::Catalog,
    filesystem,
    fixture_copy::{CopyFixture, Stage},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = CopyFixture::new()?;
    let catalog = Catalog::open(fixture.probe_path().join("working.stillharbour"))?;
    let summary = catalog.summary()?;
    println!(
        "SQLite {} / catalog schema {}",
        summary.sqlite_version, summary.schema
    );
    catalog.snapshot(fixture.probe_path().join("Family.stillharbour"))?;
    println!("Consistent independent catalog snapshot: verified");
    let volume = filesystem::volume_probe(fixture.probe_path())?;
    println!(
        "Volume query: {} total bytes / name max {:?}; physical independence unknown",
        volume.total_bytes, volume.native_name_max
    );
    for stage in [
        Stage::Intent,
        Stage::Created,
        Stage::Partial,
        Stage::Verified,
        Stage::Published,
        Stage::Recorded,
    ] {
        let copy = CopyFixture::new()?;
        copy.run_until(stage)?;
        println!(
            "Interruption at {stage:?}: {:?}; originals preserved={}",
            copy.recover()?,
            copy.source_is_preserved()?
        );
    }
    println!(
        "Fixture-only spike. No production executor, recovery evidence, or metadata-fidelity claim."
    );
    Ok(())
}
