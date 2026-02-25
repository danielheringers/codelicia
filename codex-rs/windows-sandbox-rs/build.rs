fn main() {
    println!("cargo:rerun-if-changed=codex-windows-sandbox-setup.rc");
    println!("cargo:rerun-if-changed=codex-windows-sandbox-setup.manifest");

    // Avoid linking this package resource blob when built as a dependency,
    // which can collide with the final binary resources (e.g. Tauri app).
    if std::env::var_os("CARGO_PRIMARY_PACKAGE").is_none() {
        return;
    }

    let mut res = winres::WindowsResource::new();
    // Use a custom RC with manifest only to avoid injecting VERSIONINFO.
    res.set_resource_file("codex-windows-sandbox-setup.rc");
    let _ = res.compile();
}
