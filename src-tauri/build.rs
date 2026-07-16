fn main() {
    tauri_build::build();

    // Tauri links its generated Windows resource only to application binaries. Mock-runtime
    // tests still link Common Controls v6 APIs, so their test executable needs the same manifest
    // resource to load before any IPC assertion can run.
    if matches!(std::env::var("CARGO_CFG_TARGET_OS"), Ok(target) if target == "windows") {
        let output_directory = std::env::var("OUT_DIR").expect("Cargo provides OUT_DIR");
        println!("cargo:rustc-link-search=native={output_directory}");
        println!(
            "cargo:rustc-link-arg-tests={}",
            std::path::Path::new(&output_directory)
                .join("resource.lib")
                .display()
        );
    }
}
