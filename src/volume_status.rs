use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct VolumeStatus {
    pub version: String,
    pub volumes: Vec<Volume>
}


#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Volume {
    pub id: u64,
    pub size: u64,
    pub rep_type: String,
    pub collection: String,
    pub version: String,
    pub file_count: u64,
    pub delete_count: u64,
    pub deleted_byte_count: u64,
    pub read_only: bool
}

