use crate::error::Error;
use crate::try_buf::TryBuf;

use bytes::Bytes;
use std::convert::TryFrom;

#[derive(Debug, PartialEq)]
pub struct Handle {
    pub id: u32,
    pub handle: String,
}

impl TryFrom<&mut Bytes> for Handle {
    type Error = Error;

    fn try_from(handle_bytes: &mut Bytes) -> Result<Self, Self::Error> {
        let id = handle_bytes.try_get_u32()?;
        let handle = handle_bytes.try_get_string()?;

        Ok(Handle { id, handle })
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use crate::try_buf::TryBufMut;

    use bytes::{BufMut, BytesMut};

    #[test]
    fn test_parse_handle() {
        let mut handle_bytes = BytesMut::new();

        handle_bytes.put_u32(0x01); // id
        handle_bytes.try_put_str("HANDLE").unwrap(); // handle

        assert_eq!(
            Handle::try_from(&mut handle_bytes.freeze()),
            Ok(Handle {
                id: 0x01,
                handle: String::from("HANDLE")
            })
        )
    }

    #[test]
    fn test_parse_handle_with_invalid_id() {
        let mut handle_bytes = BytesMut::new();

        handle_bytes.put_u8(0x01); // bad id

        assert_eq!(
            Handle::try_from(&mut handle_bytes.freeze()),
            Err(Error::BadMessage)
        )
    }

    #[test]
    fn test_parse_handle_with_invalid_handle() {
        let mut handle_bytes = BytesMut::new();

        handle_bytes.put_u32(0x01); // id
        handle_bytes.put_u32(0x01); // bad handle length

        assert_eq!(
            Handle::try_from(&mut handle_bytes.freeze()),
            Err(Error::BadMessage)
        )
    }
}
