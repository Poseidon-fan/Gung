use anyhow::bail;
use tokio_util::{
    bytes::{BufMut, BytesMut},
    codec::{Decoder, Encoder},
};

use crate::{AuthReq, AuthResp};

pub struct AuthReqCodec;

pub struct AuthRespCodec;

impl Encoder<AuthReq> for AuthReqCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: AuthReq, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let data = serde_json::to_vec(&item)?;
        let data = data.as_slice();
        let data_len = data.len();

        dst.reserve(data_len + 4);
        dst.put_u32(data_len as u32);
        dst.extend_from_slice(data);
        Ok(())
    }
}

impl Decoder for AuthReqCodec {
    type Error = anyhow::Error;
    type Item = AuthReq;

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
        match serde_json::from_slice::<AuthReq>(&frame_bytes[4..]) {
            Ok(frame) => Ok(Some(frame)),
            Err(e) => bail!("failed to decode auth req: {}", e),
        }
    }
}

impl Encoder<AuthResp> for AuthRespCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: AuthResp, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let data = serde_json::to_vec(&item)?;
        let data = data.as_slice();
        let data_len = data.len();

        dst.reserve(data_len + 4);
        dst.put_u32(data_len as u32);
        dst.extend_from_slice(data);
        Ok(())
    }
}

impl Decoder for AuthRespCodec {
    type Error = anyhow::Error;
    type Item = AuthResp;

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
        match serde_json::from_slice::<AuthResp>(&frame_bytes[4..]) {
            Ok(frame) => Ok(Some(frame)),
            Err(e) => bail!("failed to decode auth resp: {}", e),
        }
    }
}
