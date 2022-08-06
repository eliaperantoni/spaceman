use std::path::Path;
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use futures::Stream;
use prost_reflect::{DescriptorPool, DynamicMessage, MethodDescriptor};
use prost_reflect::prost::Message;
use prost_reflect::prost_types::FileDescriptorSet;
use tonic::{Request, Response};
use tonic::client::Grpc;
use tonic::codec::Streaming;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Uri};

use crate::{DynamicCodec, PathAndQuery};

pub struct TlsConfig {
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

    pub async fn connect(&mut self, host: &str, tls: TlsConfig) -> Result<()> {
        let host = if host.starts_with("http://") {
            host.to_string()
        } else {
            String::from("http://") + host
        };

        let uri = Uri::from_str(&host)?;

        let endpoint = Channel::builder(uri);
        let endpoint = if let Some(ca_cert) = tls.ca_cert {
            let pem = tokio::fs::read(ca_cert).await?;
            let cert = Certificate::from_pem(pem);
            endpoint.tls_config(ClientTlsConfig::new().ca_certificate(cert))?
        } else {
            endpoint
        };

        let transport = endpoint.connect().await?;
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
            S: Stream<Item=DynamicMessage> + Send + 'static,
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
            S: Stream<Item=DynamicMessage> + Send + 'static,
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
