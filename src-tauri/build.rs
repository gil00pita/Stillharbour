fn main() {
    let commands: &'static [&'static str] =
        if std::env::var_os("CARGO_FEATURE_DESKTOP_TEST").is_some() {
            &[
                "foundation_request",
                "library_request",
                "test_library_fixture",
                "test_scroll_catalog",
                "test_performance",
            ]
        } else {
            &["foundation_request", "library_request"]
        };
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(commands)),
    )
    .expect("Tauri manifest must build");
}
