use std::path::Path;

use anyhow::{Context, Result};
use prost_reflect::{
    DescriptorPool, MethodDescriptor, prost::Message, prost_types::FileDescriptorSet,
};

use blossom_types::repo::{MethodView, RepoView, ServiceView};

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
        let map_method = |method: MethodDescriptor| -> MethodView {
            MethodView {
                name: method.name().to_string(),
                is_client_streaming: method.is_client_streaming(),
                is_server_streaming: method.is_server_streaming(),
            }
        };

        let services = self.pool.services().map(|service| {
            ServiceView {
                full_name: service.full_name().to_string(),
                parent_file: service.parent_file().name().to_string(),
                methods: service.methods().map(map_method).collect(),
            }
        }).collect();

        RepoView {
            services
        }
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
