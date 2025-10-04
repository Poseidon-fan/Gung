use anyhow::Ok;
use anyhow::Result;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::PrivateKeyDer;
use rustls::pki_types::pem::PemObject;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use transport::LogicConnection;
use transport::QuicTransport;
use transport::Transport;
use transport::option::QuicTransportServerOption;
use transport::option::TlsServerOption;

#[tokio::main]
async fn main() -> Result<()> {
    let t = QuicTransport {};
    let listener = t
        .bind(
            "127.0.0.1:7777",
            QuicTransportServerOption {
                tls: TlsServerOption {
                    cert: CertificateDer::pem_file_iter("./.cert/cert.pem")?
                        .collect::<Result<_, _>>()?,
                    key: PrivateKeyDer::from_pem_file("./.cert/key.pem").unwrap(),
                },
            },
        )
        .await?;
    println!("listener bound");

    loop {
        let (mut raw_conn, remote_addr) = t.accept(&listener).await?;
        println!("accepted connection from {}", remote_addr);

        raw_conn.write_all(b"write by raw_conn").await?;
        let mut buf = vec![0; 1024];
        let n = raw_conn.read(&mut buf).await?;
        if n == 0 {
            println!("remote closed");
            break;
        }
        println!("read by raw_conn: {}", String::from_utf8_lossy(&buf));

        let conn = t.establish(raw_conn, true)?;
        println!("established connection");
        tokio::spawn(handle_connection(conn));
    }

    Ok(())
}

async fn handle_connection(conn: impl LogicConnection) -> Result<()> {
    loop {
        let stream = conn.accept().await?;
        println!("accepted stream");
        let (mut reader, mut writer) = io::split(stream);
        io::copy(&mut reader, &mut writer).await?;
    }
}
