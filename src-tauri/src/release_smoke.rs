//! Explicit, isolated packaged acceptance. Fixture mode never falls through to the network.
use std::path::PathBuf;

use crate::anilist::{
    AniListRequest, AniListResponse, AniListTransport, TransportError, TransportFuture,
};

pub struct FixtureTransport {
    pub root: PathBuf,
}

impl AniListTransport for FixtureTransport {
    fn send(&self, request: AniListRequest) -> TransportFuture<'_> {
        Box::pin(async move {
            if self.root.join("offline").exists() {
                return Err(TransportError::NetworkDisabled);
            }
            // Neither operation names nor media kinds originate from filesystem paths in IPC.
            let kind = match request.variables["mediaType"].as_str() {
                Some("MANGA") => "MANGA",
                _ => "ANIME",
            };
            let filename = format!("{}-{kind}.json", request.operation_name);
            let body = tokio::fs::read_to_string(self.root.join(filename))
                .await
                .map_err(|error| TransportError::Unavailable(format!("Smoke fixture: {error}")))?;
            Ok(AniListResponse {
                status: 200,
                headers: Default::default(),
                body,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fixture_transport_reads_only_kind_scoped_files_and_fails_closed() {
        let directory = tempfile::tempdir().expect("fixture directory");
        let transport = FixtureTransport {
            root: directory.path().to_owned(),
        };
        let request = AniListRequest {
            operation_name: "SearchMedia",
            query: "unused fixture query",
            variables: serde_json::json!({"mediaType": "MANGA"}),
        };
        assert!(transport.send(request.clone()).await.is_err());
        std::fs::write(directory.path().join("SearchMedia-MANGA.json"), "fixture")
            .expect("write fixture");
        assert_eq!(
            transport.send(request.clone()).await.expect("fixture").body,
            "fixture"
        );
        std::fs::write(directory.path().join("offline"), "").expect("offline flag");
        assert_eq!(
            transport.send(request).await,
            Err(TransportError::NetworkDisabled)
        );
    }
}
