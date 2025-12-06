use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_file = PathBuf::from(&crate_dir)
        .join("FireSimFFI.h");

    // Load cbindgen.toml config
    let config_path = PathBuf::from(&crate_dir).join("cbindgen.toml");
    let config = cbindgen::Config::from_file(&config_path)
        .expect("Failed to load cbindgen.toml");

    // Generate C bindings using cbindgen with config
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate C bindings")
        .write_to_file(output_file);

    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");
}
