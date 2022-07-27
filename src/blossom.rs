use prost_reflect::DescriptorPool;
use prost_reflect::prost_types::FileDescriptorSet;
use prost_reflect::prost::Message;

use anyhow::Result;

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
        let content = std::fs::read(path)?;
        let file_desc_set = FileDescriptorSet::decode(&content[..])?;
        self.pool.add_file_descriptor_set(file_desc_set)?;
        Ok(())
    }
}
