use crate::error::Error;
use bytes::Buf;
use std::convert::TryFrom;

#[derive(Debug, PartialEq)]
pub struct Realpath {}

impl TryFrom<&[u8]> for Realpath {
    type Error = Error;

    fn try_from(item: &[u8]) -> Result<Self, Self::Error> {
        let mut bytes = item;

        if bytes.remaining() < 1 {
            return Err(Error::BadMessage);
        }

        Ok(Realpath {})
    }
}

impl Realpath {
    pub fn parse_bytes(byte: &[u8]) -> Result<Realpath, Error> {
        Err(Error::Failure)
    }
}
