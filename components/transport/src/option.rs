use quinn::rustls::pki_types::{CertificateDer, PrivateKeyDer};

pub struct QuicTransportServerOption {
    pub tls: TlsServerOption,
}

pub struct QuicTransportClientOption {
    pub tls: TlsClientOption,
}

pub struct TlsServerOption {
    pub cert: Vec<CertificateDer<'static>>,
    pub key: PrivateKeyDer<'static>,
}

pub struct TlsClientOption {
    pub cert: Option<Vec<CertificateDer<'static>>>,
    pub hostname: Option<String>,
}
