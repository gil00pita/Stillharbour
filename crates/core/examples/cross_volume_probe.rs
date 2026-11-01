fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args_os().len() != 1 {
        return Err("Use the owned cross-volume harness; no path arguments are accepted.".into());
    }
    println!(
        "{}",
        serde_json::to_string(&stillharbour_core::executor::run_cross_volume_fixture_probe()?)?
    );
    Ok(())
}
