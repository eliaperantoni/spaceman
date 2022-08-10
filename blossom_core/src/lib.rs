use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use futures::Stream;
use http::Uri;
use http::uri::PathAndQuery;
use hyper::Client;
use hyper::client::HttpConnector;
use hyper::service::Service;
use hyper_rustls::{ConfigBuilderExt, HttpsConnector, HttpsConnectorBuilder};
pub use prost_reflect::{DynamicMessage, MethodDescriptor};
use prost_reflect::DescriptorPool;
use prost_reflect::prost::Message;
use prost_reflect::prost_types::FileDescriptorSet;
use rustls::{ClientConfig, RootCertStore};
use tonic::{Request, Response};
use tonic::body::BoxBody;
use tonic::client::Grpc;
use tonic::codec::Streaming;
pub use tonic::IntoRequest;
pub use tonic::metadata::MetadataMap;
use tower::ServiceExt;

pub use metadata::parse_metadata;

use crate::codec::DynamicCodec;

mod ca_verifier;
mod codec;
mod metadata;

pub struct Blossom {
    pool: DescriptorPool,
    conn: Option<Grpc<Client<HttpsConnector<HttpConnector>, BoxBody>>>,
}

pub struct TlsOptions {
    /// Skip verification of server's identity
    pub no_check: bool,
    /// Path to trusted CA certificate
    pub ca_cert: Option<String>,
}

fn make_rustls_config(tls_options: TlsOptions) -> Result<ClientConfig> {
    let tls = ClientConfig::builder().with_safe_defaults();

    let tls = if tls_options.no_check {
        tls.with_custom_certificate_verifier(Arc::new(
            crate::ca_verifier::DangerousCertificateVerifier,
        ))
            .with_no_client_auth()
    } else if let Some(ca_cert) = tls_options.ca_cert {
        let f = std::fs::File::open(&ca_cert)?;
        let mut f_buf = std::io::BufReader::new(f);

        let certs = rustls_pemfile::certs(&mut f_buf)?;

        let mut roots = RootCertStore::empty();
        roots.add_parsable_certificates(&certs);

        tls.with_root_certificates(roots).with_no_client_auth()
    } else {
        tls.with_native_roots().with_no_client_auth()
    };

    Ok(tls)
}

impl Default for Blossom {
    fn default() -> Self {
        Blossom::new()
    }
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
        let uri = Uri::builder()
            .scheme(if tls_options.is_some() {
                http::uri::Scheme::HTTPS
            } else {
                http::uri::Scheme::HTTP
            })
            .authority(authority)
            .path_and_query(PathAndQuery::from_static("/"))
            .build()?;

        let connector = if let Some(tls_options) = tls_options {
            let rustls_config =
                tokio::task::spawn_blocking(move || make_rustls_config(tls_options)).await??;
            HttpsConnectorBuilder::new().with_tls_config(rustls_config)
        } else {
            // Just give it a default HTTPS config, we're going to use HTTP anyways
            HttpsConnectorBuilder::new().with_native_roots()
        };

        let mut connector = connector.https_or_http().enable_http2().wrap_connector({
            let mut http_connector = HttpConnector::new();
            http_connector.enforce_http(false);
            http_connector
        });

        // Test connection so that user is eagerly notified of any errors before typing out the
        // request's body
        {
            connector.ready().await.map_err(|err| anyhow!(err))?;
            connector
                .call(uri.clone())
                .await
                .map_err(|err| anyhow!(err))?;
        }

        let transport = Client::builder().http2_only(true).build(connector);
        let client = Grpc::with_origin(transport, uri);
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
        let mut conn = self.conn.clone().ok_or_else(|| anyhow!("disconnected"))?;

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
            S: Stream<Item=DynamicMessage> + Send + 'static,
    {
        let mut conn = self.conn.clone().ok_or_else(|| anyhow!("disconnected"))?;

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
        let mut conn = self.conn.clone().ok_or_else(|| anyhow!("disconnected"))?;

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
            S: Stream<Item=DynamicMessage> + Send + 'static,
    {
        let mut conn = self.conn.clone().ok_or_else(|| anyhow!("disconnected"))?;

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
        .rsplit_once('.')
        .ok_or_else(|| anyhow!("invalid method path"))?;
    Ok(PathAndQuery::from_str(&format!(
        "/{}/{}",
        namespace, method_name
    ))?)
}
