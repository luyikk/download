fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/favicon.ico");
    // ── Embed icon into Windows .exe ─────────────────────────────────────────
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/favicon.ico");
        res.compile().expect("failed to compile Windows resources");
    }
}
