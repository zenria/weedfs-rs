use crate::types::Location;
use serde::Deserialize;
use std::fmt::Debug;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AssignResponse {
    pub count: u64,
    pub fid: String,
    pub public_url: String,
    pub url: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LookupResponse {
    pub locations: Vec<Location>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WriteResponse {
    pub size: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ResponseError {
    #[error("Weedfs error: {0}")]
    WeedError(String),
    #[error("Unexpected response")]
    UnexpectedResponse,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Response<T> {
    #[serde(flatten)]
    response: Option<T>,
    error: Option<String>,
}

impl<'de, T: Deserialize<'de>> Response<T> {
    pub fn to_result(self) -> Result<T, ResponseError> {
        if let Some(error) = self.error {
            Err(ResponseError::WeedError(error))
        } else {
            match self.response {
                None => Err(ResponseError::UnexpectedResponse),
                Some(ok) => Ok(ok),
            }
        }
    }
}
