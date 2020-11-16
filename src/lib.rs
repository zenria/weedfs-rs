use crate::types::requests::AssignRequest;
use crate::types::responses;
use crate::types::responses::{AssignResponse, LookupResponse, WriteResponse};
use anyhow::anyhow;
use anyhow::Context;
use futures::TryStreamExt;
use futures_util::StreamExt;
use reqwest::{Body, Client};
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use types::Location;
use url::Url;

pub mod error;
/// Expose types mostly used for deserialization of weed-fs responses
pub mod types;

mod utils;

pub use bytes::Bytes;
pub use reqwest;
pub use utils::get_volume_id;

/// Low-level WeedFS client.
pub struct WeedFSClient {
    master_url: Url,
    http_client: Client,
}

pub type Result<T> = std::result::Result<T, error::WeedFSError>;

impl WeedFSClient {
    pub fn new(master_url: &str) -> Result<Self> {
        Self::custom(
            master_url,
            reqwest::ClientBuilder::new().connect_timeout(Duration::from_secs(5)),
        )
    }

    pub fn custom(master_url: &str, client_builder: reqwest::ClientBuilder) -> Result<Self> {
        Ok(Self {
            master_url: Url::parse(master_url).context("Unable to parse master url")?,
            http_client: client_builder
                // always force user-agent
                .user_agent("weedfs-rs/1")
                .build()
                .context("Unable to build http client")?,
        })
    }

    pub async fn lookup(&self, volume_id: u64) -> Result<LookupResponse> {
        let lookup_url = self
            .master_url
            .join(&format!("/dir/lookup?volumeId={}", volume_id))
            .context("lookup url building")?;
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

    pub async fn assign(&self, assign_request: AssignRequest) -> Result<AssignResponse> {
        let mut path = format!("/dir/assign?count={}", assign_request.version);

        if let Some(replication) = assign_request.replication_strategy {
            path.push_str(&format!("&replication={}", replication));
        }

        if let Some(collection) = assign_request.collection {
            path.push_str(&format!("&collection={}", collection));
        }

        let assign_url = self.master_url.join(&path).context("Unable to build url")?;
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

    pub async fn write_stream<T: Into<Body>>(
        &self,
        assign_response: &AssignResponse,
        stream: T,
        filename: String,
    ) -> Result<u64> {
        let part =
            reqwest::multipart::Part::stream(stream).file_name(utils::sanitize_filename(filename));
        let form = reqwest::multipart::Form::new().part("file", part);
        let response = self
            .http_client
            .post(utils::get_write_url(&assign_response)?)
            .multipart(form)
            .send()
            .await
            .context("unable to upload file")?;
        let response = response
            .json::<responses::Response<WriteResponse>>()
            .await
            .context("unable to read body")?;
        Ok(response.to_result()?.size)
    }

    pub async fn write_file(
        &self,
        assign_response: &AssignResponse,
        mut file: File,
        filename: String,
    ) -> Result<u64> {
        let meta = file.metadata().context("unable to read file metadata")?;
        if !meta.is_file() {
            Err(anyhow!("Not a file!"))?;
        }
        if meta.len() == 0 {
            Err(anyhow!("0-sized file is not allowed!"))?;
        }

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .context("unable to read file")?;

        self.write_stream(assign_response, bytes, filename).await
    }

    /// streaming read
    pub async fn streaming_read(
        &self,
        fid: &str,
        location: &Location,
    ) -> Result<impl futures::Stream<Item = reqwest::Result<Bytes>>> {
        let read_url = utils::get_read_url(fid, location)?;

        let response = self
            .http_client
            .get(read_url)
            .send()
            .await
            .context("unable to get file")?;
        if response.status() == 404 {
            Err(error::ErrorKind::NotFound)?;
        }
        let response = response
            .error_for_status()
            .context("volume server returned an error")?;

        Ok(response.bytes_stream())
    }

    /// Convenient method to read to a Vec<u8>
    pub async fn read_to_vec(&self, fid: &str, location: &Location) -> Result<Vec<u8>> {
        self.streaming_read(fid, location)
            .await?
            .try_fold(Vec::new(), |mut buf, bytes| async move {
                buf.extend_from_slice(&bytes);
                Ok(buf)
            })
            .await
            .map_err(anyhow::Error::from)
            .map_err(error::WeedFSError::from)
    }
}

#[cfg(test)]
mod test {
    use crate::types::requests::AssignRequestBuilder;
    use crate::types::Replication;
    use crate::{utils, WeedFSClient};
    use std::fs::File;
    use std::io::Read;

    fn get_master_url() -> String {
        dotenv::dotenv().expect("Error reading .env file");
        std::env::var("MASTER_URL").expect("MASTER_URL env variable is needed for running test")
    }

    #[tokio::test]
    async fn test_assign_write_lookup_and_read() {
        let cli = WeedFSClient::new(&get_master_url()).expect("Unable to build weed client");
        let assign = cli
            .assign(
                AssignRequestBuilder::new()
                    .collection("upload-original".to_string())
                    .replication(Replication::OnceOnDifferentRack)
                    .build(),
            )
            .await
            .expect("Unable to assign");
        dbg!(&assign);

        let cargo = File::open("Cargo.toml").unwrap();
        let byte_writted = cli
            .write_file(&assign, cargo, "Cargo.toml".to_string())
            .await
            .expect("Unable to write file");
        dbg!(byte_writted);

        let lookup = cli
            .lookup(utils::get_volume_id(&assign.fid).unwrap())
            .await
            .expect("Unable to lookup for file");
        dbg!(&lookup);

        let read_cargo = cli
            .read_to_vec(&assign.fid, &lookup.locations[0])
            .await
            .expect("Unable to read file");

        let mut cargo_from_hdd = Vec::new();
        File::open("Cargo.toml")
            .unwrap()
            .read_to_end(&mut cargo_from_hdd)
            .unwrap();
        println!(
            "Read from weedfs: \n{}",
            String::from_utf8_lossy(&read_cargo)
        );
        assert_eq!(read_cargo, cargo_from_hdd);
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
        let notfound = cli.lookup(786754564231340654).await;
        assert!(notfound.is_err());
        println!("{:?}", notfound);
    }
}
