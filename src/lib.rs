use crate::requests::AssignRequest;
use crate::responses::{AssignResponse, Location, LookupResponse, WriteResponse};
use anyhow::anyhow;
use anyhow::Context;
use bytes::Bytes;
use futures_util::StreamExt;
use reqwest::{Body, Client};
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use url::Url;

pub mod dir_status;
pub mod requests;
pub mod responses;
pub mod volume_status;

mod utils;

/// Low-level WeedFS client.
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
    ) -> anyhow::Result<u64> {
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
    ) -> anyhow::Result<u64> {
        let meta = file.metadata()?;
        if !meta.is_file() {
            return Err(anyhow!("Not a file!"));
        }
        if meta.len() == 0 {
            return Err(anyhow!("0-sized file is not allowed!"));
        }

        // build multipart shit
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        self.write_stream(assign_response, bytes, filename).await
    }

    /// streaming read
    pub async fn streaming_read(
        &self,
        fid: &str,
        location: &Location,
    ) -> anyhow::Result<impl futures::Stream<Item = reqwest::Result<Bytes>>> {
        let read_url = utils::get_read_url(fid, location)?;

        let response = self
            .http_client
            .get(read_url)
            .send()
            .await
            .context("unable to get file")?
            .error_for_status()
            .context("volume server returned an error")?;

        Ok(response.bytes_stream())
    }
    /// Convenient method to read to a Vec<u8>
    pub async fn read_to_vec(&self, fid: &str, location: &Location) -> anyhow::Result<Vec<u8>> {
        let mut stream = self.streaming_read(fid, location).await?;
        let mut ret = Vec::new();
        while let Some(bytes) = stream.next().await {
            ret.extend_from_slice(&bytes?);
        }
        Ok(ret)
    }
}

#[cfg(test)]
mod test {
    use crate::dir_status::Replication;
    use crate::requests::AssignRequestBuilder;
    use crate::{utils, WeedFSClient};
    use std::fs::File;
    use std::io::Read;

    fn get_master_url() -> String {
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
        assert!(cli.lookup(786754564231340654).await.is_err());
    }
}
