use std::fmt;
use std::marker::PhantomData;

use byteorder::{BigEndian, ByteOrder};
use bytes::{Buf, BufMut, BytesMut};

use failure::{Error, ResultExt};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json as json;

fn json_string_encode(msg: String, dst: &mut BytesMut) -> Result<(), Error> {
    let msg_ref: &[u8] = msg.as_ref();

    dst.reserve(msg_ref.len() + 2);
    dst.put_u16(msg_ref.len() as u16);
    dst.put(msg_ref);

    Ok(())
}

fn json_decode<T: DeserializeOwned>(src: &mut BytesMut) -> Result<Option<T>, Error> {
    let size = {
        if src.len() < 2 {
            return Ok(None);
        }
        BigEndian::read_u16(src.as_ref()) as usize
    };

    if src.len() >= size + 2 {
        src.advance(2);
        let buf = src.split_to(size);
        Ok(Some(json::from_slice::<T>(&buf)?))
    } else {
        Ok(None)
    }
}

pub struct Encoder<Out> {
    outbound_message: PhantomData<Out>,
}

impl<Out> Encoder<Out> {
    pub fn new() -> Self {
        Self {
            outbound_message: PhantomData,
        }
    }
}

impl<Out: Serialize + fmt::Debug> tokio_util::codec::Encoder<Out> for Encoder<Out> {
    type Error = Error;

    fn encode(&mut self, msg: Out, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = json::to_string(&msg).context(format!("Couldn't Encode Message: {:?}", msg))?;
        json_string_encode(msg, dst)
    }
}

pub struct Decoder<In> {
    inbound_message: PhantomData<In>,
}

impl<In> Decoder<In> {
    pub fn new() -> Self {
        Self {
            inbound_message: PhantomData,
        }
    }
}

impl<In: DeserializeOwned> tokio_util::codec::Decoder for Decoder<In> {
    type Item = In;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        json_decode(src)
    }
}