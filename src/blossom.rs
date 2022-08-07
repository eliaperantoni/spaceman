use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use futures::Stream;
use hyper::client::connect::Connect;
use hyper_rustls::ConfigBuilderExt;
use prost_reflect::prost::Message;
use prost_reflect::prost_types::FileDescriptorSet;
use prost_reflect::{DescriptorPool, DynamicMessage, MethodDescriptor};
use rustls::RootCertStore;
use tonic::client::{Grpc, GrpcService};
use tonic::codec::Streaming;
use tonic::transport::{Certificate, Channel, ClientTlsConfig};
use tonic::{Request, Response};

use crate::{DynamicCodec, PathAndQuery};

pub struct TlsOptions {
    /// Skip verification of server's identity
    pub(crate) no_check: bool,
    /// Path to trusted CA certificate
    pub(crate) ca_cert: Option<String>,
}

pub struct Blossom {
    pool: DescriptorPool,
    conn: Option<Grpc<Channel>>,
}

impl Blossom {
    pub fn new() -> Blossom {
        Blossom {
            pool: DescriptorPool::new(),
            conn: None,
        }
    }

    pub fn pool(&self) -> &DescriptorPool {
        &self.pool
    }

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

    pub async fn connect(
        &mut self,
        authority: &str,
        tls_options: Option<TlsOptions>,
    ) -> Result<()> {
        let uri = http::Uri::builder()
            .scheme(if tls_options.is_some() {
                http::uri::Scheme::HTTPS
            } else {
                http::uri::Scheme::HTTP
            })
            .authority(authority)
            .path_and_query(PathAndQuery::from_static("/"))
            .build()?;

        let builder = Channel::builder(uri);

        let mut transport = if let Some(tls_options) = tls_options {
            let tls = rustls::ClientConfig::builder().with_safe_defaults();
            let tls = if tls_options.no_check {
                tls.with_custom_certificate_verifier(Arc::new(
                    crate::ca_verifier::DangerousCertificateVerifier,
                ))
                .with_no_client_auth()
            } else {
                if let Some(ca_cert) = tls_options.ca_cert {
                    let pem = tokio::fs::read(ca_cert).await?;
                    let certs = rustls_pemfile::certs(&mut &pem[..])?;

                    let mut roots = RootCertStore::empty();
                    roots.add_parsable_certificates(&certs);

                    tls.with_root_certificates(roots).with_no_client_auth()
                } else {
                    tls.with_native_roots().with_no_client_auth()
                }
            };

            let https = hyper_rustls::HttpsConnectorBuilder::new()
                .with_tls_config(tls)
                .https_only()
                .enable_http2()
                .build();

            builder.connect_with_connector(https).await?
        } else {
            builder.connect().await?
        };

        let client = Grpc::new(transport);
        self.conn = Some(client);

        Ok(())
    }

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

    pub async fn unary(
        &self,
        md: &MethodDescriptor,
        req: Request<DynamicMessage>,
    ) -> Result<Response<DynamicMessage>> {
        let mut conn = self.conn.clone().ok_or(anyhow!("disconnected"))?;

        conn.ready().await?;

        let path = method_desc_to_path(md)?;
        let codec = DynamicCodec::new(md.clone());

        conn.unary(req, path, codec).await.map_err(|err| err.into())
    }

    pub async fn client_streaming<S>(
        &self,
        md: &MethodDescriptor,
        req: Request<S>,
    ) -> Result<Response<DynamicMessage>>
    where
        S: Stream<Item = DynamicMessage> + Send + 'static,
    {
        let mut conn = self.conn.clone().ok_or(anyhow!("disconnected"))?;

        conn.ready().await?;

        let path = method_desc_to_path(md)?;
        let codec = DynamicCodec::new(md.clone());

        conn.client_streaming(req, path, codec)
            .await
            .map_err(|err| err.into())
    }

    pub async fn server_streaming(
        &self,
        md: &MethodDescriptor,
        req: Request<DynamicMessage>,
    ) -> Result<Response<Streaming<DynamicMessage>>> {
        let mut conn = self.conn.clone().ok_or(anyhow!("disconnected"))?;

        conn.ready().await?;

        let path = method_desc_to_path(md)?;
        let codec = DynamicCodec::new(md.clone());

        conn.server_streaming(req, path, codec)
            .await
            .map_err(|err| err.into())
    }

    pub async fn bidi_streaming<S>(
        &self,
        md: &MethodDescriptor,
        req: Request<S>,
    ) -> Result<Response<Streaming<DynamicMessage>>>
    where
        S: Stream<Item = DynamicMessage> + Send + 'static,
    {
        let mut conn = self.conn.clone().ok_or(anyhow!("disconnected"))?;

        conn.ready().await?;

        let path = method_desc_to_path(md)?;
        let codec = DynamicCodec::new(md.clone());

        conn.streaming(req, path, codec)
            .await
            .map_err(|err| err.into())
    }
}

fn method_desc_to_path(md: &MethodDescriptor) -> Result<PathAndQuery> {
    let full_name = md.full_name();
    let (namespace, method_name) = full_name
        .rsplit_once(".")
        .ok_or(anyhow!("invalid method path"))?;
    Ok(PathAndQuery::from_str(&format!(
        "/{}/{}",
        namespace, method_name
    ))?)
}
