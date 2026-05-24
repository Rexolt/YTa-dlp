fn main() {
    // Embed the build-time target triple so we can find dev-mode sidecar binaries
    // that are suffixed with it (e.g. `yt-dlp-x86_64-unknown-linux-gnu`).
    if let Ok(triple) = std::env::var("TARGET") {
        println!("cargo:rustc-env=TARGET_TRIPLE={triple}");
    }
    tauri_build::build()
}
