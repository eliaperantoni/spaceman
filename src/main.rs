use std::path::Path;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use futures::StreamExt;
use http::uri::PathAndQuery;
use prost_reflect::{DynamicMessage, MethodDescriptor};
use serde::Serialize;
use serde_json::{Deserializer, Serializer};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio_stream::wrappers::ReceiverStream;
use tonic::codegen::http;
use tonic::IntoRequest;

use codec::DynamicCodec;

mod blossom;
mod codec;

#[derive(Parser)]
#[clap(author, version, about)]
#[clap(propagate_version = true)]
struct Options {
    /// Comma separated list of paths to Protobuf descriptor files
    #[clap(value_parser, value_name = "DESCRIPTORS")]
    descriptors: String,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List available methods
    List,
    /// Perform a call to a method
    Call {
        /// Host to communicate with. Usually something like `schema://ip:port`
        #[clap(value_parser, value_name = "HOST")]
        host: String,
        /// Full name of the method to invoke. Usually something like `package.service.name`
        #[clap(value_parser, value_name = "METHOD")]
        method: String,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let options: Options = Options::parse();

    let mut b = blossom::Blossom::new();

    for descriptor_path in options.descriptors.split(",") {
        b.add_descriptor(&Path::new(descriptor_path))
            .context("adding descriptor")?;
    }

    match options.command {
        Command::List => (),
        Command::Call { host, method } => {
            b.connect(&host).await?;

            let md = b
                .find_method_desc(&method)
                .ok_or(anyhow!("couldn't find method"))?;

            match (md.is_client_streaming(), md.is_server_streaming()) {
                (false, false) => unary(&b, &md).await?,
                (true, false) => client_streaming(&b, &md).await?,
                (false, true) => server_streaming(&b, &md).await?,
                (true, true) => bidi_streaming(&b, &md).await?,
            };
        }
    };

    Ok(())
}

async fn unary(b: &blossom::Blossom, md: &MethodDescriptor) -> Result<()> {
    let mut de = Deserializer::from_reader(std::io::stdin());
    let req_msg =
        DynamicMessage::deserialize(md.input(), &mut de).context("parsing request body")?;

    let req = req_msg.into_request();

    let res = b.unary(md, req).await?;

    let mut se = Serializer::pretty(std::io::stdout());
    res.get_ref().serialize(&mut se)?;
    println!();

    Ok(())
}

async fn client_streaming(b: &blossom::Blossom, md: &MethodDescriptor) -> Result<()> {
    let (rx, mut t_error_rx) = spawn_stdin_reader(md);
    let req = ReceiverStream::new(rx).into_request();

    let res = tokio::select! {
        // If reader thread encountered an error. Note that the pattern match only fails if the
        // thread quit without sending an error, which means all is good.
        Ok(err) = &mut t_error_rx => {
            Err(err)
        },
        res = b.client_streaming(md, req) => {
            res
        }
    }?;

    let mut se = Serializer::pretty(std::io::stdout());
    res.get_ref().serialize(&mut se)?;
    println!();

    Ok(())
}

async fn server_streaming(b: &blossom::Blossom, md: &MethodDescriptor) -> Result<()> {
    let mut de = Deserializer::from_reader(std::io::stdin());
    let req_msg =
        DynamicMessage::deserialize(md.input(), &mut de).context("parsing request body")?;

    let req = req_msg.into_request();

    let mut res = b.server_streaming(md, req).await?;
    let stream = res.get_mut();

    let mut se = Serializer::pretty(std::io::stdout());
    while let Some(msg) = stream.next().await {
        let msg = msg?;
        msg.serialize(&mut se)?;
        println!();
    }

    Ok(())
}

async fn bidi_streaming(b: &blossom::Blossom, md: &MethodDescriptor) -> Result<()> {
    let (rx, mut t_error_rx) = spawn_stdin_reader(md);
    let req = ReceiverStream::new(rx).into_request();

    let res = tokio::select! {
        // If reader thread encountered an error. Note that the pattern match only fails if the
        // thread quit without sending an error, which means all is good.
        Ok(err) = &mut t_error_rx => {
            Err(err)
        },
        res = b.bidi_streaming(md, req) => {
            res
        }
    }?;

    let mut res: tonic::Response<tonic::codec::Streaming<DynamicMessage>> = res;
    let stream = res.get_mut();

    let mut se = Serializer::pretty(std::io::stdout());

    loop {
        tokio::select! {
            // If reader thread encountered an error. Note that the pattern match only fails if the
            // thread quit without sending an error, which means all is good.
            Ok(err) = &mut t_error_rx => {
                return Err(err);
            },
            msg = stream.next() => {
                if let Some(msg) = msg {
                    let msg = msg?;
                    msg.serialize(&mut se)?;
                    println!();
                } else {
                    break;
                }
            }
        };
    }

    Ok(())
}

fn spawn_stdin_reader(
    md: &MethodDescriptor,
) -> (
    mpsc::Receiver<DynamicMessage>,
    oneshot::Receiver<anyhow::Error>,
) {
    // Used to send parsed messages from the thread reading from STDIN to the thread running the
    // gRPC client
    let (tx, rx) = mpsc::channel::<DynamicMessage>(10);

    // Used by the thread reading from STDIN to communicate any error on its part
    let (t_error_tx, t_error_rx) = oneshot::channel();

    let input_type = md.input();
    // WARN It's not possible to stop this thread so if things go wrong on the gRPC side, this is
    //  left leaking and blocking on an STDIN read. Whenever this `client_streaming` function
    //  returns `Err`, the program should be terminated
    std::thread::spawn(move || {
        let mut de = Deserializer::from_reader(std::io::stdin());
        loop {
            let req_msg = match DynamicMessage::deserialize(input_type.clone(), &mut de) {
                Ok(req_msg) => req_msg,
                Err(err) => {
                    if err.is_eof() {
                        // This will cause `tx` to be dropped which, in turn, will commit the stream
                        break;
                    }
                    let err = anyhow!(err).context("parsing message");
                    let _ = t_error_tx.send(err);
                    break;
                }
            };
            tx.blocking_send(req_msg)
                .expect("couldn't send message down internal channel");
        }
    });

    return (rx, t_error_rx);
}
