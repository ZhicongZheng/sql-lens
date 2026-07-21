fn main() {
    println!("cargo:rerun-if-changed=../sql-lens-app/web/dist");
    println!("cargo:rerun-if-changed=../sql-lens-app/web/dist/index.html");

    let embed = std::env::var_os("CARGO_FEATURE_EMBEDDED_UI").is_some();
    if !embed {
        return;
    }

    let index = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sql-lens-app/web/dist/index.html");
    if !index.is_file() {
        panic!(
            "feature `embedded-ui` requires a built SPA at crates/sql-lens-app/web/dist/index.html.\n\
             Run `./scripts/build-web.sh` from the repository root, then rebuild."
        );
    }
}
