use crate::requests::AssignRequest;
use crate::responses::{AssignResponse, LookupResponse};
use anyhow::Context;
use reqwest::Client;
use std::time::Duration;
use url::Url;

pub mod dir_status;
pub mod requests;
pub mod responses;
pub mod volume_status;

pub struct WeedFSClient {
    master_url: Url,
    http_client: Client,
}

impl WeedFSClient {
    pub fn new(master_url: &str) -> anyhow::Result<Self> {
        Ok(Self {
            master_url: Url::parse(master_url).context("Unable to parse master url")?,
            http_client: reqwest::ClientBuilder::new()
                .connect_timeout(Duration::from_secs(5))
                .user_agent("weedfs-rs/1")
                .build()
                .context("Unable to build http client")?,
        })
    }

    pub async fn lookup(&self, volume_id: u64) -> anyhow::Result<LookupResponse> {
        let lookup_url = self
            .master_url
            .join(&format!("/dir/lookup?volumeId={}", volume_id))?;
        let response = self
            .http_client
            .get(lookup_url)
            .send()
            .await
            .context("lookup request failed")?;
        let response = response
            .json::<responses::Response<LookupResponse>>()
            .await
            .context("lookup response body read failed")?;
        Ok(response.to_result()?)
    }

    pub async fn assign(&self, assign_request: AssignRequest) -> anyhow::Result<AssignResponse> {
        let mut path = format!("/dir/assign?count={}", assign_request.version);

        if let Some(replication) = assign_request.replication_strategy {
            path.push_str(&format!("&replication={}", replication));
        }

        if let Some(collection) = assign_request.collection {
            path.push_str(&format!("&collection={}", collection));
        }

        let assign_url = self.master_url.join(&path)?;
        dbg!(&assign_url);
        let response = self
            .http_client
            .get(assign_url)
            .send()
            .await
            .context("assign request failed")?;
        let response = response
            .json::<responses::Response<AssignResponse>>()
            .await
            .context("assign response body read failed")?;
        Ok(response.to_result()?)
    }
}

#[cfg(test)]
mod test {
    use crate::dir_status::Replication;
    use crate::requests::AssignRequestBuilder;
    use crate::WeedFSClient;

    fn get_master_url() -> String {
        std::env::var("MASTER_URL").expect("MASTER_URL env variable is needed for running test")
    }

    #[tokio::test]
    async fn test_assign() {
        let cli = WeedFSClient::new(&get_master_url()).expect("Unable to build weed client");
        dbg!(cli
            .assign(
                AssignRequestBuilder::new()
                    .collection("upload-original".to_string())
                    .replication(Replication::OnceOnDifferentRack)
                    .build(),
            )
            .await
            .expect("Unable to assign"));
    }

    #[tokio::test]
    async fn test_lookup() {
        let cli = WeedFSClient::new(&get_master_url()).expect("Unable to build weed client");
        assert_eq!(
            cli.lookup(7400)
                .await
                .expect("request failed!")
                .locations
                .iter()
                .count(),
            2,
        );
        assert!(cli.lookup(786754564231340654).await.is_err());
    }
}
