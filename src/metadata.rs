use std::fmt::Formatter;
use std::str::FromStr;

use anyhow::Result;
use serde::de::{Error, MapAccess, Visitor};
use serde::Deserializer;
use tonic::metadata::errors::{
    InvalidMetadataKey, InvalidMetadataValue, InvalidMetadataValueBytes,
};
use tonic::metadata::{Ascii, Binary, MetadataKey, MetadataMap, MetadataValue};

struct MetadataVisitor;

impl<'de> Visitor<'de> for MetadataVisitor {
    type Value = MetadataMap;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "a map")
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut metadata = MetadataMap::new();

        while let Some((k, v)) = map.next_entry::<String, String>()? {
            let is_bin = k.ends_with("-bin");

            if is_bin {
                let k: MetadataKey<Binary> = FromStr::from_str(k.as_str())
                    .map_err(|err: InvalidMetadataKey| Error::custom(err.to_string()))?;
                let v: MetadataValue<Binary> = TryFrom::try_from(v.as_bytes())
                    .map_err(|err: InvalidMetadataValueBytes| Error::custom(err.to_string()))?;

                metadata.append_bin(k, v);
            } else {
                let k: MetadataKey<Ascii> = FromStr::from_str(k.as_str())
                    .map_err(|err: InvalidMetadataKey| Error::custom(err.to_string()))?;
                let v: MetadataValue<Ascii> = TryFrom::try_from(v.as_str())
                    .map_err(|err: InvalidMetadataValue| Error::custom(err.to_string()))?;

                metadata.append(k, v);
            }
        }

        Ok(metadata)
    }
}

pub fn parse_metadata<S: AsRef<str>>(metadata: S) -> Result<MetadataMap> {
    let mut de = serde_json::Deserializer::from_str(metadata.as_ref());
    de.deserialize_map(MetadataVisitor)
        .map_err(|err| err.into())
}
