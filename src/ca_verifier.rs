use std::time::SystemTime;

use rustls::client::{ServerCertVerified, ServerCertVerifier};
use rustls::{Certificate, Error, ServerName};

pub(crate) struct DangerousCertificateVerifier;

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
