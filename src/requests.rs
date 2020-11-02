use crate::dir_status::Replication;

#[derive(Debug)]
pub struct AssignRequest {
    pub(crate) version: u64,
    pub(crate) replication_strategy: Option<Replication>,
    pub(crate) collection: Option<String>,
}

pub struct AssignRequestBuilder(AssignRequest);

impl AssignRequestBuilder {
    pub fn new() -> Self {
        Self(AssignRequest {
            version: 1,
            replication_strategy: None,
            collection: None,
        })
    }

    pub fn version(mut self, version: u64) -> Self {
        self.0.version = version;
        self
    }
    pub fn replication(mut self, replication: Replication) -> Self {
        self.0.replication_strategy = Some(replication);
        self
    }
    pub fn collection(mut self, collection: String) -> Self {
        self.0.collection = Some(collection);
        self
    }

    pub fn build(self) -> AssignRequest {
        self.0
    }
}
