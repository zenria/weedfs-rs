use crate::types::responses::AssignResponse;
use crate::types::Location;
use anyhow::anyhow;
use anyhow::Context;
use url::Url;

pub fn get_write_url(assign_response: &AssignResponse) -> anyhow::Result<Url> {
    get_file_url(&assign_response.fid, &assign_response.public_url)
}

pub fn get_read_url(fid: &str, location: &Location) -> anyhow::Result<Url> {
    get_file_url(fid, &location.public_url)
}

fn get_file_url(fid: &str, location_public_url: &str) -> anyhow::Result<Url> {
    let mut url = String::new();
    if !location_public_url.starts_with("http") {
        url.push_str("http://");
    }
    url.push_str(&location_public_url);

    url.push_str("/");
    url.push_str(&fid);

    // FIXME here we should append some version on the end of the file id if the
    //       version is > 0; unfortunately, I do not known to what it corresponds to...
    //       Maybe the "count" field of the assign response... who knows?...

    Url::parse(&url).context("Unable to build write url")
}

pub fn sanitize_filename(filename: String) -> String {
    if filename.len() == 0 {
        "file".to_string()
    } else if filename.len() > 255 {
        filename[0..255].to_string()
    } else {
        filename
    }
}

pub fn get_volume_id(fid: &str) -> anyhow::Result<u64> {
    match fid.find(",") {
        None => Err(anyhow!(
            "{} does not seems to be a valid weedfs file id",
            fid
        )),
        Some(pos) => fid[0..pos]
            .parse()
            .context("Unable to parse volume id from file id"),
    }
}
