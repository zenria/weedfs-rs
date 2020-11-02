use crate::types::Replication;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DirStatus {
    pub topology: Topology,
    pub version: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Topology {
    pub data_centers: Vec<DataCenter>,
    pub free: u32,
    pub max: u32,
    #[serde(rename = "layouts")]
    pub layouts: Vec<Layout>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DataCenter {
    pub id: Option<String>,
    pub free: u32,
    pub max: u32,
    pub racks: Vec<Rack>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Rack {
    pub id: Option<String>,
    pub data_nodes: Option<Vec<DataNode>>,
    pub free: u32,
    pub max: u32,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DataNode {
    pub free: u32,
    pub max: u32,
    pub public_url: String,
    pub volumes: u32,
}

#[derive(Deserialize, Debug)]
pub struct Layout {
    pub collection: String,
    pub replication: Replication,
    pub writables: Vec<u32>,
}
