// Links the prebuilt liblitert-lm.so for the optional in-process FFI path (M6).
// Only active with `--features ffi`. The default build (subprocess) needs nothing.
fn main() {
    if std::env::var("CARGO_FEATURE_FFI").is_ok() {
        // Dir containing liblitert-lm.so (override with LITERT_LM_LIB_DIR).
        let default = format!(
            "{}/.local/share/uv/tools/litert-lm/lib/python3.12/site-packages/litert_lm",
            std::env::var("HOME").unwrap_or_default()
        );
        let dir = std::env::var("LITERT_LM_LIB_DIR").unwrap_or(default);
        println!("cargo:rustc-link-search=native={dir}");
        println!("cargo:rustc-link-lib=dylib=litert-lm");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
        println!("cargo:rerun-if-env-changed=LITERT_LM_LIB_DIR");
    }
}
