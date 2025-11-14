#![allow(dead_code)]
use anyhow::Result;
use anyhow::bail;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use transport::LogicConnection;
use transport::QuicTransport;
use transport::QuicTransportClientOption;
use transport::TcpTransport;
use transport::TcpTransportClientOption;
use transport::Transport;

#[tokio::main]
async fn main() -> Result<()> {
    handle_tcp().await
    // handle_quic().await
}

async fn handle_quic() -> Result<()> {
    let t = QuicTransport {};

    let mut raw_conn = t
        .connect(
            "127.0.0.1:7777",
            QuicTransportClientOption {
                cert: Some(
                    CertificateDer::pem_file_iter("./.cert/cert.pem")?.collect::<Result<_, _>>()?,
                ),
                hostname: Some("localhost".to_string()),
            },
        )
        .await?;
    println!("connected to server");
    raw_conn.write_all(b"write by raw_conn").await?;
    let mut buf = vec![0; 1024];
    let n = raw_conn.read(&mut buf).await?;
    if n == 0 {
        println!("remote closed");
        bail!("remote closed");
    }
    println!("read by raw_conn: {}", String::from_utf8_lossy(&buf));

    let conn = t.establish(raw_conn, false)?;
    println!("established connection");

    handle_connection(conn).await?;

    Ok(())
}

async fn handle_tcp() -> Result<()> {
    let t = TcpTransport { no_delay: true };

    let mut raw_conn = t
        .connect("127.0.0.1:7777", TcpTransportClientOption {})
        .await?;
    println!("connected to server");
    raw_conn.write_all(b"write by raw_conn").await?;
    let mut buf = vec![0; 1024];
    let n = raw_conn.read(&mut buf).await?;
    if n == 0 {
        println!("remote closed");
        bail!("remote closed");
    }
    println!("read by raw_conn: {}", String::from_utf8_lossy(&buf));

    let conn = t.establish(raw_conn, false)?;
    println!("established connection");

    handle_connection(conn).await?;

    Ok(())
}

async fn handle_connection(conn: impl LogicConnection) -> Result<()> {
    for i in 0..7 {
        let mut stream = conn.open().await?;
        println!("opened stream");
        stream
            .write_all(format!("This is Sream {i}").as_bytes())
            .await?;
        stream.flush().await?;
        let mut buf = vec![0; 1024];
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            println!("remote closed");
            break;
        }
        println!("recv: {}", String::from_utf8_lossy(&buf));
    }
    Ok(())
}
