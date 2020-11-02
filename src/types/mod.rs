use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub mod dir_status;
pub mod requests;
pub mod responses;
pub mod volume_status;

#[derive(Deserialize, Serialize, Debug)]
pub enum Replication {
    /// no replication
    #[serde(rename = "000")]
    None,
    /// replicate once on the same rack
    #[serde(rename = "001")]
    OnceOnSameRack,
    /// replicate once on a different rack, but same data center
    #[serde(rename = "010")]
    OnceOnDifferentRack,
    /// replicate once on a different data center
    #[serde(rename = "100")]
    OnceOnDifferentDC,
    /// replicate twice on two different data center
    #[serde(rename = "200")]
    TwiceOnDifferentDC,
    /// replicate once on a different rack, and once on a different data center
    #[serde(rename = "110")]
    OnceOnDifferentRackAndOnceOnDifferentDC,
}

impl Display for Replication {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.serialize(f)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub public_url: String,
    pub url: String,
}
