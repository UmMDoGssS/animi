use std::{future::Future, pin::Pin};

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct AniListRequest {
    pub operation_name: &'static str,
    pub variables: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AniListResponse {
    pub status: u16,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    NetworkDisabled,
    Unavailable(String),
}

pub type TransportFuture<'a> =
    Pin<Box<dyn Future<Output = Result<AniListResponse, TransportError>> + Send + 'a>>;

/// HTTP remains injectable so tests cannot accidentally contact AniList.
pub trait AniListTransport: Send + Sync {
    fn send(&self, request: AniListRequest) -> TransportFuture<'_>;
}

/// Safe default for tests and code paths that have not explicitly installed a client.
#[derive(Debug, Default)]
pub struct NetworkDisabledTransport;

impl AniListTransport for NetworkDisabledTransport {
    fn send(&self, _request: AniListRequest) -> TransportFuture<'_> {
        Box::pin(async { Err(TransportError::NetworkDisabled) })
    }
}

#[cfg(test)]
mod tests {
    use super::{AniListRequest, AniListTransport, NetworkDisabledTransport, TransportError};

    #[tokio::test]
    async fn default_test_transport_cannot_reach_the_network() {
        let result = NetworkDisabledTransport
            .send(AniListRequest {
                operation_name: "Smoke",
                variables: serde_json::json!({}),
            })
            .await;

        assert_eq!(result, Err(TransportError::NetworkDisabled));
    }
}
