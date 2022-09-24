use std::path::Path;

use anyhow::{Context, Result};
use prost_reflect::{
    prost::Message, prost_types::FileDescriptorSet, DescriptorPool, MethodDescriptor,
    ServiceDescriptor,
};
use serde::Serialize;

/// Stores protobuf descriptors.
#[derive(Default, Clone)]
pub struct Repo {
    pool: DescriptorPool,
}

impl Repo {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Repo::default()
    }

    #[allow(dead_code)]
    pub fn add_descriptor(&mut self, path: &Path) -> Result<()> {
        // Read whole file descriptor set to bytes vec
        let content = std::fs::read(path).context("reading file descriptor set")?;
        // Decode it
        let file_desc_set =
            FileDescriptorSet::decode(&content[..]).context("decoding file descriptor set")?;
        // And add it to the pool
        self.pool
            .add_file_descriptor_set(file_desc_set)
            .context("adding file descriptor set to pool")?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn view(&self) -> RepoView {
        self.into()
    }

    #[allow(dead_code)]
    pub fn find_method_desc(&self, full_name: &str) -> Option<MethodDescriptor> {
        let service = self
            .pool
            .services()
            .find(|service| full_name.starts_with(service.full_name()))?;
        let method = service
            .methods()
            .find(|method| method.full_name() == full_name)?;
        Some(method)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RepoView {
    pub services: Vec<ServiceView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceView {
    pub full_name: String,
    pub parent_file: String,
    pub methods: Vec<MethodView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MethodView {
    pub name: String,
    pub is_client_streaming: bool,
    pub is_server_streaming: bool,
}

impl From<&'_ Repo> for RepoView {
    fn from(repo: &Repo) -> Self {
        RepoView {
            services: repo.pool.services().map(Into::into).collect(),
        }
    }
}

impl From<ServiceDescriptor> for ServiceView {
    fn from(service: ServiceDescriptor) -> Self {
        ServiceView {
            full_name: service.full_name().to_string(),
            parent_file: service.parent_file().name().to_string(),
            methods: service.methods().map(Into::into).collect(),
        }
    }
}

impl From<MethodDescriptor> for MethodView {
    fn from(method: MethodDescriptor) -> Self {
        MethodView {
            name: method.name().to_string(),
            is_client_streaming: method.is_client_streaming(),
            is_server_streaming: method.is_server_streaming(),
        }
    }
}
