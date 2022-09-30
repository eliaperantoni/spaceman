use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use prost_reflect::{
    prost::Message, prost_types::FileDescriptorSet, DescriptorPool, MethodDescriptor,
};

use blossom_types::repo::{MethodView, RepoView, Serial, ServiceView};

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
    pub fn view(&self) -> (RepoView, MethodLut) {
        let mut services = Vec::new();
        let mut lut = MethodLut::default();

        for service in self.pool.services() {
            let mut methods = Vec::new();

            for method in service.methods() {
                methods.push({
                    let mut mv = MethodView {
                        name: method.name().to_string(),
                        full_name: method.full_name().to_string(),
                        input_msg_name: method.input().name().to_string(),
                        output_msg_name: method.output().name().to_string(),
                        is_client_streaming: method.is_client_streaming(),
                        is_server_streaming: method.is_server_streaming(),
                        serial: 0, // Temporary value
                    };
                    let serial = lut.add(method);
                    mv.serial = serial;
                    mv
                });
            }

            services.push(ServiceView {
                name: service.name().to_string(),
                full_name: service.full_name().to_string(),
                parent_file: service.parent_file().name().to_string(),
                methods,
            })
        }

        (RepoView { services }, lut)
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

#[derive(Default)]
pub struct MethodLut {
    storage: HashMap<Serial, MethodDescriptor>,
    serial: Serial,
}

impl MethodLut {
    fn add(&mut self, md: MethodDescriptor) -> Serial {
        self.serial += 1;
        self.storage.insert(self.serial, md);
        self.serial
    }

    pub fn lookup(&self, serial: Serial) -> Option<&MethodDescriptor> {
        self.storage.get(&serial)
    }
}
