fn main() {
    println!("cargo:rerun-if-changed=../../web/build");
    built::write_built_file().expect("Failed to acquire build-time information");
}
