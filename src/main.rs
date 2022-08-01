use std::path::Path;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use http::uri::PathAndQuery;
use prost_reflect::DynamicMessage;
use serde::Serialize;
use serde_json::{Deserializer, Serializer};
use tonic::codegen::http;
use tonic::IntoRequest;

use codec::DynamicCodec;

mod blossom;
mod codec;

#[derive(Parser)]
#[clap(author, version, about)]
struct Options {
    /// Host to communicate with. Usually something like `schema://ip:port`.
    #[clap(value_parser, value_name = "HOST")]
    host: String,
    /// Path to a Protobuf descriptor file. If there's more than one, they should be comma separated
    #[clap(value_parser, value_name = "DESCRIPTOR")]
    descriptors: String,
    /// Full name of the method to invoke. Usually something like `package.service.name`
    #[clap(value_parser, value_name = "METHOD")]
    method: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let options: Options = Options::parse();

    let mut b = blossom::Blossom::new();

    for descriptor_path in options.descriptors.split(",") {
        b.add_descriptor(&Path::new(descriptor_path)).
            context("adding descriptor")?;
    }

    b.connect(&options.host).await?;

    let md = b.find_method_desc(&options.method).
        ok_or(anyhow!("couldn't find method"))?;

    let mut de = Deserializer::from_reader(std::io::stdin());
    let req_msg = DynamicMessage::deserialize(md.input(), &mut de).
        context("parsing request body")?;
    de.end().context("message completed, expected end of file")?;

    let req = req_msg.into_request();

    let res = b.unary(&md, req).await?;

    let mut se = Serializer::pretty(std::io::stdout());
    res.get_ref().serialize(&mut se)?;

    Ok(())
}
