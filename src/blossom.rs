use prost_reflect::DescriptorPool;
use prost_reflect::prost_types::FileDescriptorSet;
use prost_reflect::prost::Message;

use anyhow::{Context, Result};

use std::path::Path;

pub struct Blossom {
    pool: DescriptorPool,
}

impl Blossom {
    pub fn new() -> Blossom {
        Blossom {
            pool: DescriptorPool::new()
        }
    }

    pub fn add_descriptor(&mut self, path: &Path) -> Result<()> {
        // Read whole file descriptor set to bytes vec
        let content = std::fs::read(path)
            .context("reading file descriptor set")?;
        // Decode it
        let file_desc_set = FileDescriptorSet::decode(&content[..])
            .context("decoding file descriptor set")?;
        // And add it to the pool
        self.pool.add_file_descriptor_set(file_desc_set)
            .context("adding file descriptor set to pool")?;
        Ok(())
    }
}
