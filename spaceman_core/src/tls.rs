use std::sync::Arc;
use std::time::SystemTime;

use anyhow::Result;
use hyper_rustls::ConfigBuilderExt;
use rustls::client::{ServerCertVerified, ServerCertVerifier};
use rustls::{Certificate, ClientConfig, Error, RootCertStore, ServerName};

use spaceman_types::endpoint::TlsOptions;

pub fn make_rustls_config(tls_options: &TlsOptions) -> Result<ClientConfig> {
    let tls = ClientConfig::builder().with_safe_defaults();

    let tls = if tls_options.no_check {
        tls.with_custom_certificate_verifier(Arc::new(DangerousCertificateVerifier))
            .with_no_client_auth()
    } else if let Some(ca_cert) = &tls_options.ca_cert {
        let f = std::fs::File::open(ca_cert)?;
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

pub struct DangerousCertificateVerifier;

impl ServerCertVerifier for DangerousCertificateVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &Certificate,
        _intermediates: &[Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: SystemTime,
    ) -> Result<ServerCertVerified, Error> {
        Ok(ServerCertVerified::assertion())
    }
}
