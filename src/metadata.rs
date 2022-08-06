use std::str::FromStr;

use anyhow::{anyhow, Result};
use tonic::metadata::{Ascii, Binary, MetadataKey, MetadataMap, MetadataValue};

pub fn parse_metadata(metadata: Vec<String>) -> Result<MetadataMap> {
    let mut result = MetadataMap::new();
    for entry in metadata {
        let (k, v) = entry
            .split_once(":")
            .ok_or(anyhow!("badly formatted metadata entry"))?;

        let is_binary = k.ends_with("-bin");

        if is_binary {
            let k: MetadataKey<Binary> = FromStr::from_str(k)?;
            let v: MetadataValue<Binary> = TryFrom::try_from(v.as_bytes())?;

            result.append_bin(k, v);
        } else {
            let k: MetadataKey<Ascii> = FromStr::from_str(k)?;
            let v: MetadataValue<Ascii> = TryFrom::try_from(v)?;

            result.append(k, v);
        }
    }
    Ok(result)
}
