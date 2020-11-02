use anyhow::Context;
use reqwest::Client;
use std::time::Duration;
use url::Url;

pub mod dir_status;
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

    pub async fn lookup(&self, volume_id: u64) -> anyhow::Result<responses::LookupResponse> {
        let lookup_url = self
            .master_url
            .join(&format!("/dir/lookup?volumeId={}", volume_id))?;
        self.http_client
            .get(lookup_url)
            .send()
            .await
            .context("lookup request failed")?
            .json::<responses::LookupResponse>()
            .await
            .context("lookup response body parse fail")
    }
}

#[cfg(test)]
mod test {
    use crate::WeedFSClient;

    fn get_master_url() -> String {
        std::env::var("MASTER_URL").expect("MASTER_URL env variable is needed for running test")
    }

    #[tokio::test]
    async fn test_lookup() {
        let cli = WeedFSClient::new(&get_master_url()).expect("Unable to build weed client");
        assert_eq!(
            cli.lookup(7400)
                .await
                .expect("request failed!")
                .iter()
                .count(),
            2,
        );
        assert_eq!(
            cli.lookup(786754564231340654)
                .await
                .expect("request failed!")
                .iter()
                .count(),
            0,
        );
    }
}
