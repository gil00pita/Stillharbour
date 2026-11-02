fn main() {
    println!("cargo:rerun-if-changed=native/imageio.c");
    println!("cargo:rerun-if-changed=native/backup_parent_guard.c");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("native/backup_parent_guard.c")
            .flag("-std=c11")
            .flag("-mmacosx-version-min=13.0")
            .warnings_into_errors(true)
            .compile("stillharbour_backup_parent_guard");
        cc::Build::new()
            .file("native/imageio.c")
            .flag("-std=c11")
            .flag("-mmacosx-version-min=13.0")
            .warnings_into_errors(true)
            .compile("stillharbour_imageio");
        for framework in ["ImageIO", "CoreGraphics", "CoreFoundation", "ColorSync"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
    }
}
