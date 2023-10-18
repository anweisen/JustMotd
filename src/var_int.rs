use std::{io, str};
use std::io::Write;

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

      // bitwise and: 0x7F = 0111 1111 -> deletes the first bit, keeps the rest
      // first bit will be used in order to determine whether to continue
      decoded_value |= (byte as i32 & 0x7F) << (i * 7);

      // first bit mask: 0x80 = 1000 0000
      if byte & 0x80 == 0 {
        return Ok(decoded_value);
      }
    }
    Err(VarIntDecodeError::TooLarge)
  }

  pub fn encode(mut value: i32, mut writer: &mut impl Write) -> Result<(), io::Error> {
    loop {
      // write 7 bits at a time, msf is continuation bit
      let mut byte = (value & 0x7F) as u8;
      value >>= 7;

      // if there are missing bits, add continuation bit
      if value != 0 {
        byte |= 0x80;
      }

      writer.write_all(&[byte])?;

      // there are not further bits, exit
      if value == 0 {
        break;
      }
    }
    Ok(())
  }
}

pub struct VarString(pub str);

impl VarString {
  pub fn decode(mut buffer: &mut &[u8]) -> String {
    let length = VarInt::decode(&mut *buffer).expect("");
    let bytes = &buffer[..length as usize];
    buffer.advance(length as usize);
    str::from_utf8(bytes).unwrap().to_string()
  }

  pub fn encode(value: String, mut writer: &mut impl Write) -> Result<(), io::Error> {
    let bytes = value.as_bytes();
    VarInt::encode(bytes.len() as i32, &mut writer)?;
    writer.write_all(bytes)
  }
}

