#![allow(dead_code)]
use anyhow::Ok;
use anyhow::Result;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::PrivateKeyDer;
use rustls::pki_types::pem::PemObject;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use transport::LogicConnection;
use transport::QuicTransport;
use transport::TcpTransport;
use transport::Transport;
use transport::option::QuicTransportServerOption;
use transport::option::TcpTransportServerOption;
use transport::option::TlsServerOption;
use transport::option::TransportServerOption;

#[tokio::main]
async fn main() -> Result<()> {
    // handle_quic().await
    handle_tcp().await
}

async fn handle_quic() -> Result<()> {
    let t = QuicTransport {};
    let listener = t
        .bind(
            "127.0.0.1:7777",
            TransportServerOption::Quic(QuicTransportServerOption {
                tls: TlsServerOption {
                    cert: CertificateDer::pem_file_iter("./.cert/cert.pem")?
                        .collect::<Result<_, _>>()?,
                    key: PrivateKeyDer::from_pem_file("./.cert/key.pem").unwrap(),
                },
            }),
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

async fn handle_tcp() -> Result<()> {
    let t = TcpTransport {};
    let listener = t
        .bind(
            "127.0.0.1:7777",
            TransportServerOption::Tcp(TcpTransportServerOption {}),
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

        let mut buf = vec![0; 1024];
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            println!("remote closed");
            return Err(anyhow::anyhow!("remote closed"));
        }
        writer.write_all(&buf).await?;

        println!("finish stream");
    }
}
