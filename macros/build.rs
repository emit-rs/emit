fn main() {
    println!("cargo:rerun-if-env-changed=EMIT_FILTER");
    println!("cargo:rerun-if-changed=build.rs");
}
