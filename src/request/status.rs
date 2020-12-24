use crate::error::Error;
use bytes::Buf;
use std::convert::TryFrom;

#[derive(Debug, PartialEq)]
pub struct Status {}

impl TryFrom<&[u8]> for Status {
    type Error = Error;

    fn try_from(item: &[u8]) -> Result<Self, Self::Error> {
        let mut bytes = item;

        if bytes.remaining() < 1 {
            return Err(Error::BadMessage);
        }

        Ok(Status {})
    }
}

impl Status {
    pub fn parse_bytes(byte: &[u8]) -> Result<Status, Error> {
        Err(Error::Failure)
    }
}
