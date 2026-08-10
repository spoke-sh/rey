use std::{env, path::PathBuf, process::Command};

const UNKNOWN_REVISION: &str = "unknown";

fn main() {
    println!("cargo:rerun-if-env-changed=REY_BUILD_REVISION");
    let revision = env::var("REY_BUILD_REVISION")
        .ok()
        .filter(|value| valid_git_oid(value))
        .or_else(repository_revision)
        .unwrap_or_else(|| UNKNOWN_REVISION.to_owned());
    println!("cargo:rustc-env=REY_BUILD_REVISION={revision}");
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
