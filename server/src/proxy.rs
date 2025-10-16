use tokio::net::TcpStream;

use anyhow::Result;
use tokio::io;
use tokio::sync::mpsc;
use transport::LogicConnection;

pub struct Proxy<T: LogicConnection> {
    pxy_id: String,
    conn: T,
    req_rx: mpsc::UnboundedReceiver<TcpStream>,
}

impl<T: LogicConnection> Proxy<T> {
    pub async fn run(&mut self) -> Result<()> {
        while let Some(mut req_stream) = self.req_rx.recv().await {
            let mut data_channel = self.conn.open().await?;
            let _ = io::copy_bidirectional(&mut req_stream, &mut data_channel).await;
        }
        Ok(())
    }
}
