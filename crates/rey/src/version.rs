use serde::Serialize;

pub const VERSION_SCHEMA: &str = "rey.version.v1";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const COMMIT_SHA: &str = env!("REY_BUILD_REVISION");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct VersionDescriptor {
    pub schema: &'static str,
    pub version: &'static str,
    pub commit_sha: &'static str,
}

impl VersionDescriptor {
    #[must_use]
    pub const fn current() -> Self {
        Self {
            schema: VERSION_SCHEMA,
            version: VERSION,
            commit_sha: COMMIT_SHA,
        }
    }
}
