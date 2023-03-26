use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use tonic::metadata::{Ascii, Binary, MetadataKey, MetadataMap, MetadataValue};

#[derive(Default)]
pub struct Metadata {
    storage_ascii: HashMap<String, Vec<String>>,
    storage_bin: HashMap<String, Vec<Vec<u8>>>,
}

impl Metadata {
    pub fn add_ascii(&mut self, key: String, value: String) -> Result<()> {
        if key.ends_with("-bin") {
            return Err(anyhow!("ascii key must not end in '-bin'"));
        }
        self.storage_ascii.entry(key).or_default().push(value);
        Ok(())
    }

    pub fn add_bin(&mut self, key: String, value: Vec<u8>) -> Result<()> {
        if !key.ends_with("-bin") {
            return Err(anyhow!("binary key must end in '-bin'"));
        }
        self.storage_bin.entry(key).or_default().push(value);
        Ok(())
    }

    pub fn finalize(self) -> Result<MetadataMap> {
        let mut result = MetadataMap::new();

        for (key, values) in self.storage_ascii {
            let key: MetadataKey<Ascii> = FromStr::from_str(&key)?;

            for value in values {
                // Must not contain any non-ASCII (sub-128) chars
                let value: MetadataValue<Ascii> = TryFrom::try_from(&value)?;
                result.append(key.clone(), value);
            }
        }

        for (key, values) in self.storage_bin {
            let key: MetadataKey<Binary> = FromStr::from_str(&key)?;

            for value in values {
                let value: MetadataValue<Binary> = TryFrom::try_from(&value[..])?;
                result.append_bin(key.clone(), value);
            }
        }

        Ok(result)
    }
}
