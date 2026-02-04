// Build script to embed build timestamp into the binary
use std::process::Command;

fn main() {
    // Get current date/time in a readable format
    let output = Command::new("date")
        .args(["+%Y-%m-%d %H:%M:%S"])
        .output()
        .expect("Failed to get build timestamp");

    let build_time = String::from_utf8(output.stdout)
        .unwrap()
        .trim()
        .to_string();

    // Export as a compile-time environment variable
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_time);

    // Rerun build script if any source files change (ensures fresh timestamp on rebuild)
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=Cargo.toml");
}
