use std::str;

use bytes::Buf;
use tokio::io::AsyncReadExt;

#[derive(Debug)]
pub enum VarIntDecodeError {
  Incomplete,
  TooLarge,
}

pub struct VarInt(pub i32);

impl VarInt {
  pub async fn decode_partial<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<i32, VarIntDecodeError> {
    let mut decoded_value: i32 = 0;

    for i in 0..5 {
      match reader.read_u8().await {
        Ok(byte) => {
          // bitwise and: 01111111 -> deletes the first bit, keeps the rest
          // first bit will be used in order to determine whether to continue
          decoded_value |= (byte as i32 & 0b01111111) << (i * 7);

          if byte & 0b10000000 == 0 {
            return Ok(decoded_value);
          }
        }
        Err(_err) => return Err(VarIntDecodeError::Incomplete),
      }
    }
    Err(VarIntDecodeError::TooLarge)
  }

  pub fn decode(buffer: &mut &[u8]) -> Result<i32, VarIntDecodeError> {
    let mut decoded_value: i32 = 0;

    for i in 0..5 {
      let byte = buffer.get_u8();

      // bitwise and: 01111111 -> deletes the first bit, keeps the rest
      // first bit will be used in order to determine whether to continue
      decoded_value |= (byte as i32 & 0b01111111) << (i * 7);

      if byte & 0b10000000 == 0 {
        return Ok(decoded_value);
      }
    }
    Err(VarIntDecodeError::TooLarge)
  }
}

pub struct VarString;

impl VarString {
  pub fn decode(mut buffer: &mut &[u8]) -> String {
    let length = VarInt::decode(&mut *buffer).expect("");
    let bytes = &buffer[..length as usize];
    buffer.advance(length as usize);
    str::from_utf8(bytes).unwrap().to_string()
  }
}

