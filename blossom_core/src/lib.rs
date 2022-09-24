use std::str::FromStr;

use anyhow::{anyhow, Result};
use futures::Stream;
use http::uri::PathAndQuery;
use http::Uri;
use hyper::client::HttpConnector;
use hyper::Client;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
pub use prost_reflect::{DynamicMessage, MethodDescriptor, SerializeOptions};
use tonic::body::BoxBody;
use tonic::client::Grpc;
use tonic::codec::Streaming;
pub use tonic::IntoRequest;
use tonic::{Request, Response};

pub use metadata::Metadata;
pub use repo::{MethodView, Repo, RepoView, ServiceView};
pub use tls::TlsOptions;

use crate::codec::DynamicCodec;

mod codec;
mod metadata;
mod repo;
mod tls;

/// Descriptor for a gRPC server.
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// Host name plus port.
    pub authority: String,
    /// TLS options.
    pub tls: Option<TlsOptions>,
}

/// A gRPC connection.
pub struct Conn(Grpc<Client<HttpsConnector<HttpConnector>, BoxBody>>);

impl Conn {
    #[allow(dead_code)]
    pub fn new(ep: &Endpoint) -> Result<Self> {
        let uri = Uri::builder()
            .scheme(if ep.tls.is_some() {
                http::uri::Scheme::HTTPS
            } else {
                http::uri::Scheme::HTTP
            })
            .authority(ep.authority.clone())
            .path_and_query(PathAndQuery::from_static("/"))
            .build()?;

        let rustls_config = if let Some(tls) = &ep.tls {
            tls::make_rustls_config(tls)
        } else {
            // It shouldn't matter all that much what config we give here because the
            // `HttpsConnector` should just forward any request to `HttpConnector` because of the
            // scheme defined above.
            tls::make_rustls_config(&TlsOptions {
                no_check: true,
                ca_cert: None,
            })
        }?;

        let connector = HttpsConnectorBuilder::new().with_tls_config(rustls_config);
        let connector = connector.https_or_http().enable_http2().wrap_connector({
            let mut http_connector = HttpConnector::new();
            http_connector.enforce_http(false);
            http_connector
        });

        let transport = Client::builder().http2_only(true).build(connector);
        let client = Grpc::with_origin(transport, uri);

        Ok(Self(client))
    }

    pub async fn unary(
        &self,
        md: &MethodDescriptor,
        req: Request<DynamicMessage>,
    ) -> Result<Response<DynamicMessage>> {
        let mut conn = self.0.clone();

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
        let mut conn = self.0.clone();

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
        let mut conn = self.0.clone();

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
        let mut conn = self.0.clone();

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
