fn main() {
    let mut attributes = tauri_build::Attributes::new();

    // tauri-build only embeds the Common-Controls v6 manifest into the app
    // binary, so test binaries (e.g. `export_bindings`) crash on start with
    // STATUS_ENTRYPOINT_NOT_FOUND. Embed it into every target instead.
    let windows_msvc = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if windows_msvc {
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        let manifest = std::env::current_dir()
            .unwrap()
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }

    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
