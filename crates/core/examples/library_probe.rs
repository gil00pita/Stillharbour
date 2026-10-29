#[cfg(target_os = "macos")]
fn main() {
    let observations = [100_000, 1_000_000]
        .into_iter()
        .map(|rows| {
            stillharbour_core::library::probe::run(rows)
                .expect("Owned synthetic catalog probe must succeed")
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&observations).expect("Observation JSON")
    );
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Synthetic native probe requires the macOS adapter; other platforms are unverified.");
}
