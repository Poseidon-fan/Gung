// Commands transferred through data channel

use std::net::SocketAddr;

use anyhow::bail;
use bincode::{Decode, Encode};
use tokio_util::{
    bytes::{BufMut, BytesMut},
    codec::{Decoder, Encoder},
};

#[derive(Encode, Decode)]
pub enum ServerCommand {
    ForwardingStarted(SocketAddr),
    ForwardingFailed(String),
    ForwardingShutdown,
}

#[derive(Encode, Decode)]
pub enum ClientCommand {
    ClientShutdown,
}

pub struct ServerCommandCodec;

pub struct ClientCommandCodec;

impl Encoder<ServerCommand> for ServerCommandCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: ServerCommand, dst: &mut BytesMut) -> Result<(), Self::Error> {
        const CFG: bincode::config::Configuration = bincode::config::standard();
        let data = bincode::encode_to_vec(&item, CFG)?;
        let data = data.as_slice();
        let data_len = data.len();

        dst.reserve(data_len + 4);
        dst.put_u32(data_len as u32);
        dst.extend_from_slice(data);
        Ok(())
    }
}

impl Decoder for ServerCommandCodec {
    type Error = anyhow::Error;
    type Item = ServerCommand;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let buf_len = src.len();

        if buf_len < 4 {
            return Ok(None);
        }

        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let data_len = u32::from_be_bytes(length_bytes) as usize;

        let frame_len = data_len + 4;

        if buf_len < frame_len {
            src.reserve(frame_len - buf_len);
            return Ok(None);
        }

        let frame_bytes = src.split_to(frame_len);
        const CFG: bincode::config::Configuration = bincode::config::standard();
        match bincode::decode_from_slice::<ServerCommand, _>(&frame_bytes[4..], CFG) {
            Ok((frame, _)) => Ok(Some(frame)),
            Err(e) => bail!("failed to decode auth req: {}", e),
        }
    }
}

impl Encoder<ClientCommand> for ClientCommandCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: ClientCommand, dst: &mut BytesMut) -> Result<(), Self::Error> {
        const CFG: bincode::config::Configuration = bincode::config::standard();
        let data = bincode::encode_to_vec(&item, CFG)?;
        let data = data.as_slice();
        let data_len = data.len();

        dst.reserve(data_len + 4);
        dst.put_u32(data_len as u32);
        dst.extend_from_slice(data);
        Ok(())
    }
}

impl Decoder for ClientCommandCodec {
    type Error = anyhow::Error;
    type Item = ClientCommand;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let buf_len = src.len();
        if buf_len < 4 {
            return Ok(None);
        }

        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let data_len = u32::from_be_bytes(length_bytes) as usize;

        let frame_len = data_len + 4;

        if buf_len < frame_len {
            src.reserve(frame_len - buf_len);
            return Ok(None);
        }

        let frame_bytes = src.split_to(frame_len);
        const CFG: bincode::config::Configuration = bincode::config::standard();
        match bincode::decode_from_slice::<ClientCommand, _>(&frame_bytes[4..], CFG) {
            Ok((frame, _)) => Ok(Some(frame)),
            Err(e) => bail!("failed to decode client command: {}", e),
        }
    }
}
