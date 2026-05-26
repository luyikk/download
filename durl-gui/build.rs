use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=lang/");
    println!("cargo:rerun-if-changed=assets/letter-d.ico");

    // ── Copy lang/ to target output dir ──────────────────────────────────────
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let target_dir = Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .expect("failed to find target dir");

    let lang_src = Path::new("lang");
    let lang_dst = target_dir.join("lang");

    if lang_src.exists() {
        std::fs::create_dir_all(&lang_dst).ok();
        for entry in std::fs::read_dir(lang_src).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                let dest = lang_dst.join(entry.file_name());
                std::fs::copy(&path, &dest).ok();
            }
        }
    }

    // ── Embed icon into Windows .exe ─────────────────────────────────────────
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/letter-d.ico");
        res.compile().expect("failed to compile Windows resources");
    }
}
