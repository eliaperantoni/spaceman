use serde::{Deserialize, Serialize};

/// Descriptor for a gRPC server.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Endpoint {
    /// Host name plus port.
    pub authority: String,
    /// TLS options.
    pub tls: Option<TlsOptions>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsOptions {
    /// Skip verification of server's identity.
    pub no_check: bool,
    /// Path to trusted CA certificate.
    pub ca_cert: Option<String>,
}
