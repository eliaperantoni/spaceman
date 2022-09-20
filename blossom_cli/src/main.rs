use std::ops::Not;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use clap::{Args, Parser, Subcommand};
use colored::Colorize;
use futures::StreamExt;
use serde_json::{Deserializer, Serializer};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio_stream::wrappers::ReceiverStream;

use blossom_core::{
    Conn, DynamicMessage, Endpoint, IntoRequest, Metadata, MethodDescriptor, Repo, SerializeOptions,
};

#[derive(Parser)]
#[clap(author, version, about)]
#[clap(propagate_version = true)]
struct Options {
    /// Path to a Protobuf descriptor file. Can supply more than one
    #[clap(
        required = true,
        short,
        long = "desc",
        value_parser,
        value_name = "DESCRIPTOR"
    )]
    descriptor: Vec<String>,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Prints a tree of all loaded services with their methods
    ///
    /// The text in brackets next to the service's name is the path to the Protobuf file that
    /// produced it (relative to the compiler's root). An upwards arrow (↑ ) next to a method
    /// indicates that the client can stream multiple messages while a downwards arrow (↓ ) indicates
    /// that the server can stream multiple messages. The presence of both arrows indicates that the
    /// method is bidirectionally streaming.
    List,
    /// Perform a call to a method
    Call {
        /// Server to communicate with in `ip:port` form. Do not include the schema.
        #[clap(value_parser, value_name = "AUTHORITY")]
        authority: String,
        /// Full name of the method to invoke. Usually something like `package.service.name`
        #[clap(value_parser, value_name = "METHOD")]
        method: String,
        /// A metadata pair to include in the request formatted like `key:value`.
        ///
        /// Watch out for any whitespace inside `value` which may cause your shell to split it into
        /// multiple arguments unless escaped. The split `key` and `value` strings are used as-is
        /// and not stripped of any whitespace.
        ///
        /// Multiple values can be supplied for the same key simply by providing this flag multiple
        /// times and reusing the same key.
        ///
        /// If the name of the key ends in `-bin`, then the value is expected to a base64 encoded
        /// byte array.
        #[clap(short = 'M', long = "meta", value_parser, value_name = "METADATA")]
        metadata: Vec<String>,
        /// Disable TLS.
        #[clap(short, long)]
        insecure: bool,
        #[clap(flatten)]
        tls_options: TlsOptions,
    },
}

#[derive(Args)]
struct TlsOptions {
    /// Skip verification of server's identity.
    #[clap(long = "tls-nocheck")]
    no_check: bool,
    /// Path to trusted CA certificate for verifying the server's identity.
    #[clap(long = "tls-cacert", value_parser)]
    ca_cert: Option<String>,
}

impl From<TlsOptions> for blossom_core::TlsOptions {
    fn from(from: TlsOptions) -> Self {
        Self {
            no_check: from.no_check,
            ca_cert: from.ca_cert,
        }
    }
}

static SERIALIZE_OPTIONS: &'static SerializeOptions =
    &SerializeOptions::new().skip_default_fields(false);

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let options: Options = Options::parse();

    let mut repo = Repo::new();

    for descriptor_path in &options.descriptor {
        repo.add_descriptor(Path::new(descriptor_path))
            .context("adding descriptor")?;
    }

    match options.command {
        Command::List => {
            list(&repo);
        }
        Command::Call {
            authority,
            method,
            metadata: raw_metadata,
            insecure,
            tls_options,
        } => {
            let conn = Conn::new(&Endpoint {
                authority,
                tls: insecure.not().then_some(tls_options.into()),
            })?;

            let md = repo
                .find_method_desc(&method)
                .ok_or_else(|| anyhow!("couldn't find method"))?;

            let mut metadata = Metadata::default();
            for str in raw_metadata {
                let (key, value) = str
                    .split_once(':')
                    .ok_or_else(|| anyhow!("badly formatted metadata"))?;
                if key.ends_with("-bin") {
                    let value = base64::decode(value)?;
                    metadata.add_bin(key.to_string(), value)?;
                } else {
                    metadata.add_ascii(key.to_string(), value.to_string())?;
                }
            }

            match (md.is_client_streaming(), md.is_server_streaming()) {
                (false, false) => unary(&conn, &md, metadata).await?,
                (true, false) => client_streaming(&conn, &md, metadata).await?,
                (false, true) => server_streaming(&conn, &md, metadata).await?,
                (true, true) => bidi_streaming(&conn, &md, metadata).await?,
            };
        }
    };

    Ok(())
}

async fn unary(conn: &Conn, md: &MethodDescriptor, metadata: Metadata) -> Result<()> {
    let mut de = Deserializer::from_reader(std::io::stdin());
    let req_msg =
        DynamicMessage::deserialize(md.input(), &mut de).context("parsing request body")?;

    let mut req = req_msg.into_request();
    *req.metadata_mut() = metadata.finalize()?;

    let res = conn.unary(md, req).await?;

    let mut se = Serializer::pretty(std::io::stdout());
    res.get_ref()
        .serialize_with_options(&mut se, SERIALIZE_OPTIONS)?;
    println!();

    Ok(())
}

async fn client_streaming(conn: &Conn, md: &MethodDescriptor, metadata: Metadata) -> Result<()> {
    let (rx, mut t_error_rx) = spawn_stdin_reader(md);
    let mut req = ReceiverStream::new(rx).into_request();
    *req.metadata_mut() = metadata.finalize()?;

    let res = tokio::select! {
        // If reader thread encountered an error. Note that the pattern match only fails if the
        // thread quit without sending an error, which means all is good.
        Ok(err) = &mut t_error_rx => {
            Err(err)
        },
        res = conn.client_streaming(md, req) => {
            res
        }
    }?;

    let mut se = Serializer::pretty(std::io::stdout());
    res.get_ref()
        .serialize_with_options(&mut se, SERIALIZE_OPTIONS)?;
    println!();

    Ok(())
}

async fn server_streaming(conn: &Conn, md: &MethodDescriptor, metadata: Metadata) -> Result<()> {
    let mut de = Deserializer::from_reader(std::io::stdin());
    let req_msg =
        DynamicMessage::deserialize(md.input(), &mut de).context("parsing request body")?;

    let mut req = req_msg.into_request();
    *req.metadata_mut() = metadata.finalize()?;

    let mut res = conn.server_streaming(md, req).await?;
    let stream = res.get_mut();

    let mut se = Serializer::pretty(std::io::stdout());
    while let Some(msg) = stream.next().await {
        let msg = msg?;
        msg.serialize_with_options(&mut se, SERIALIZE_OPTIONS)?;
        println!();
    }

    Ok(())
}

async fn bidi_streaming(conn: &Conn, md: &MethodDescriptor, metadata: Metadata) -> Result<()> {
    let (rx, mut t_error_rx) = spawn_stdin_reader(md);
    let mut req = ReceiverStream::new(rx).into_request();
    *req.metadata_mut() = metadata.finalize()?;

    let mut res = tokio::select! {
        // If reader thread encountered an error. Note that the pattern match only fails if the
        // thread quit without sending an error, which means all is good.
        Ok(err) = &mut t_error_rx => {
            Err(err)
        },
        res = conn.bidi_streaming(md, req) => {
            res
        }
    }?;

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
                    msg.serialize_with_options(&mut se, SERIALIZE_OPTIONS)?;
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

    (rx, t_error_rx)
}

fn list(repo: &Repo) {
    for service in repo.pool_ref().services() {
        println!(
            "{} {}",
            service.full_name(),
            format!("[{}]", service.parent_file().name()).dimmed()
        );
        let it = service.methods();
        let len = it.len();
        for (i, method) in it.enumerate() {
            // Is last method?
            let branch = if i == len - 1 { "└─" } else { "├─" };

            println!(
                "{} {} {}{} ",
                branch.dimmed(),
                method.name(),
                if method.is_client_streaming() {
                    "↑ "
                } else {
                    ""
                }
                .cyan(),
                if method.is_server_streaming() {
                    "↓ "
                } else {
                    ""
                }
                .purple()
            );
        }
    }
}
