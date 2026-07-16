use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmokeResponse {
    pub service: &'static str,
    pub version: &'static str,
    pub message: &'static str,
}

impl SmokeResponse {
    pub fn ready() -> Self {
        Self {
            service: "animi-core",
            version: env!("CARGO_PKG_VERSION"),
            message: "Local core is ready",
        }
    }
}
