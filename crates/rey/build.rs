use std::{
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const UNKNOWN_REVISION: &str = "unknown";
const UI_ASSET_MANIFEST: &str = "rey_ui_assets.rs";

fn main() {
    println!("cargo:rerun-if-env-changed=REY_BUILD_REVISION");
    println!("cargo:rerun-if-env-changed=REY_UI_DIST_DIR");
    let revision = env::var("REY_BUILD_REVISION")
        .ok()
        .filter(|value| valid_git_oid(value))
        .or_else(repository_revision)
        .unwrap_or_else(|| UNKNOWN_REVISION.to_owned());
    println!("cargo:rustc-env=REY_BUILD_REVISION={revision}");
    generate_ui_asset_manifest();
}

fn generate_ui_asset_manifest() {
    let manifest = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo must provide CARGO_MANIFEST_DIR"),
    );
    let dist = env::var_os("REY_UI_DIST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("../../apps/agent/dist"));
    let index = dist.join("index.html");
    let assets = dist.join("assets");
    println!("cargo:rerun-if-changed={}", dist.display());

    assert!(
        index.is_file(),
        "embedded UI entry point {} does not exist; run the UI build first or set REY_UI_DIST_DIR",
        index.display()
    );

    let mut files = fs::read_dir(&assets)
        .unwrap_or_else(|error| {
            panic!(
                "embedded UI asset directory {} could not be read: {error}",
                assets.display()
            )
        })
        .map(|entry| {
            entry
                .expect("embedded UI asset directory entry must be readable")
                .path()
        })
        .filter(|path| path.is_file())
        .filter(|path| {
            matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("css" | "js")
            )
        })
        .collect::<Vec<_>>();
    files.sort();
    assert!(!files.is_empty(), "embedded UI asset directory is empty");

    let mut generated = String::new();
    writeln!(
        generated,
        "const INDEX_HTML: &[u8] = include_bytes!({:?});",
        rust_path(&index)
    )
    .expect("writing a String cannot fail");
    generated.push_str("const STATIC_UI_ASSETS: &[(&str, &[u8], &str)] = &[\n");
    for path in files {
        write_ui_asset(&mut generated, &path);
    }
    generated.push_str("];\n");

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must provide OUT_DIR"))
        .join(UI_ASSET_MANIFEST);
    fs::write(&output, generated).unwrap_or_else(|error| {
        panic!(
            "embedded UI asset manifest {} could not be written: {error}",
            output.display()
        )
    });
}

fn write_ui_asset(generated: &mut String, path: &Path) {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .expect("embedded UI asset names must be UTF-8");
    let content_type = match path.extension().and_then(|value| value.to_str()) {
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        _ => return,
    };
    writeln!(
        generated,
        "    ({:?}, include_bytes!({:?}), {:?}),",
        format!("/assets/{file_name}"),
        rust_path(path),
        content_type,
    )
    .expect("writing a String cannot fail");
}

fn rust_path(path: &Path) -> &str {
    path.to_str()
        .expect("embedded UI asset paths must be valid UTF-8")
}

fn repository_revision() -> Option<String> {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR")?);
    let workspace = manifest.parent()?.parent()?;
    register_git_input(workspace, "HEAD");
    let symbolic_ref = git_line(workspace, &["symbolic-ref", "-q", "HEAD"]);
    if let Some(reference) = symbolic_ref.as_deref() {
        register_git_input(workspace, reference);
    }
    git_line(workspace, &["rev-parse", "--verify", "HEAD^{commit}"])
        .filter(|value| valid_git_oid(value))
}

fn register_git_input(workspace: &std::path::Path, reference: &str) {
    if let Some(path) = git_line(workspace, &["rev-parse", "--git-path", reference]) {
        let path = PathBuf::from(path);
        let path = if path.is_absolute() {
            path
        } else {
            workspace.join(path)
        };
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn git_line(workspace: &std::path::Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(arguments)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn valid_git_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
