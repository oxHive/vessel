fn main() {
    // Re-embed when the built dashboard changes (release builds bake it in).
    println!("cargo:rerun-if-changed=dashboard/dist");

    let dist_index = std::path::Path::new("dashboard/dist/index.html");
    if dist_index.exists() {
        return;
    }

    // Release binaries without the dashboard are broken artifacts: rust-embed
    // embeds an empty tree and every UI route 404s (this shipped in <= 0.1.4,
    // and in oxvessel 0.1.4's crates.io package, which never included
    // dashboard/dist). Fail the build — including `cargo package`'s verify
    // step — instead.
    let profile = std::env::var("PROFILE").unwrap_or_default();
    if profile == "release" {
        panic!(
            "dashboard/dist/index.html missing — build the dashboard first (`just dashboard`), \
             otherwise the release binary ships without a UI"
        );
    }

    // Non-release builds (debug, test, clippy, tarpaulin, CI checks that
    // don't need the real UI): `#[derive(RustEmbed)] #[folder = "..."]` in
    // src/server.rs resolves its folder at compile time regardless of
    // profile, so a fresh checkout without a built dashboard fails to
    // compile at all — cargo test, clippy, and tarpaulin all hit this on
    // CI, not just `cargo build --release`. Write a placeholder so the
    // crate compiles; `vessel up` from such a build just serves a stub page
    // instead of the real UI (see the runtime check in src/server.rs).
    std::fs::create_dir_all(dist_index.parent().unwrap())
        .expect("create dashboard/dist placeholder directory");
    std::fs::write(
        dist_index,
        "<!doctype html><title>vessel (dashboard not built)</title>",
    )
    .expect("write dashboard/dist placeholder index.html");
}
