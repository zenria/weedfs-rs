use crate::requests::AssignRequest;
use crate::responses::{AssignResponse, LookupResponse, WriteResponse};
use anyhow::anyhow;
use anyhow::Context;
use reqwest::Client;
use std::fs::File;
use std::io::Read;
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

    pub async fn write(
        &self,
        assign_response: AssignResponse,
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

        let part = reqwest::multipart::Part::bytes(bytes).file_name(sanitize_filename(filename));
        let form = reqwest::multipart::Form::new().part("file", part);

        let response = self
            .http_client
            .post(get_write_url(&assign_response)?)
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
}

fn get_write_url(assign_response: &AssignResponse) -> anyhow::Result<Url> {
    let mut url = String::new();
    if !assign_response.public_url.starts_with("http") {
        url.push_str("http://");
    }
    url.push_str(&assign_response.public_url);

    url.push_str("/");
    url.push_str(&assign_response.fid);

    // FIXME here we should append some version on the end of the file id if the
    //       version is > 0; unfortunately, I do not known to what it corresponds to...
    //       Maybe the "count" field of the assign response... who knows...

    Url::parse(&url).context("Unable to build write url")
}

fn sanitize_filename(filename: String) -> String {
    if filename.len() == 0 {
        "file".to_string()
    } else if filename.len() > 255 {
        filename[0..255].to_string()
    } else {
        filename
    }
}

#[cfg(test)]
mod test {
    use crate::dir_status::Replication;
    use crate::requests::AssignRequestBuilder;
    use crate::WeedFSClient;
    use std::fs::File;

    fn get_master_url() -> String {
        std::env::var("MASTER_URL").expect("MASTER_URL env variable is needed for running test")
    }

    #[tokio::test]
    async fn test_assign_and_write() {
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
            .write(assign, cargo, "Cargo.toml".to_string())
            .await
            .expect("Unable to write file");
        dbg!(byte_writted);
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
