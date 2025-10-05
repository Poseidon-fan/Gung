use quinn::rustls::pki_types::{CertificateDer, PrivateKeyDer};

pub enum TransportServerOption {
    Quic(QuicTransportServerOption),
    Tcp(TcpTransportServerOption),
}

pub enum TransportClientOption {
    Quic(QuicTransportClientOption),
    Tcp(TcpTransportClientOption),
}

pub struct QuicTransportServerOption {
    pub tls: TlsServerOption,
}

pub struct QuicTransportClientOption {
    pub tls: TlsClientOption,
}

pub struct TcpTransportServerOption {}

pub struct TcpTransportClientOption {}

pub struct TlsServerOption {
    pub cert: Vec<CertificateDer<'static>>,
    pub key: PrivateKeyDer<'static>,
}

pub struct TlsClientOption {
    pub cert: Option<Vec<CertificateDer<'static>>>,
    pub hostname: Option<String>,
}
