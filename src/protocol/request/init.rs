use crate::error::Error;
use crate::try_buf::TryBuf;

use bytes::Bytes;
use std::convert::TryFrom;

#[derive(Debug, PartialEq)]
pub struct Init {
    pub version: u8,
}

impl TryFrom<&mut Bytes> for Init {
    type Error = Error;

    fn try_from(init_bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Init {
            version: init_bytes.try_get_u8()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bytes::{BufMut, BytesMut};

    #[test]
    fn test_parse_init_message() {
        let mut init_bytes = BytesMut::new();

        init_bytes.put_u8(0x03);

        assert_eq!(
            Init::try_from(&mut init_bytes.freeze()),
            Ok(Init { version: 0x03 })
        );
    }

    #[test]
    fn test_parse_invalid_message() {
        let init_bytes = BytesMut::new();

        assert_eq!(
            Init::try_from(&mut init_bytes.freeze()),
            Err(Error::BadMessage)
        );
    }
}
