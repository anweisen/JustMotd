use std::io;

use bytes::Buf;
use log::{error, trace};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::ComposedConfigs;
use crate::var_int::{VarInt, VarIntDecodeError, VarString, VarStringDecodeError};

pub async fn handle_client(mut stream: TcpStream, composed_configs: ComposedConfigs) {
  let mut peek_bytes = [0; 3];
  match stream.peek(&mut peek_bytes).await {
    Ok(0) => return, // probably not fully received yet
    Ok(n) => {
      // could also be packet with length 254=0xFE (unlikely, but possible)
      // https://wiki.vg/Server_List_Ping#1.6
      // https://wiki.vg/Server_List_Ping#1.4_to_1.5
      if peek_bytes[0] == 0xFE {
        handle_legacy(n, &peek_bytes, &composed_configs, &mut stream).await;
        drop(stream); // TODO
        return;
      }
    }
    Err(err) => {
      error!("Could not peek TcpStream: {}", err);
      return;
    }
  }

  let length = match VarInt::decode_partial(&mut stream).await {
    Ok(length) => length,
    Err(VarIntDecodeError::Incomplete) => return, // probably not fully received yet // TODO already above?!
    Err(VarIntDecodeError::TooLarge) => {
      error!("Unable to decode VarInt: TooLarge");
      return;
    }
  };

  trace!("Received packet prefixed with (length) {}/{:#x}", length, length);

  let mut bytes = vec![0u8; length as usize];
  match stream.read_exact(&mut bytes).await {
    Ok(_) => {}
    Err(err) => {
      drop(stream);
      error!("Unable to read from stream: {}", err);
      return;
    }
  }

  // decode handshake
  let handshake_data = match decode_handshake(&mut bytes) {
    Ok(data) => data,
    Err(err) => {
      drop(stream);
      error!("Unable to decode handshake: {:?}", err);
      return;
    }
  };

  trace!("Decoded: packet_id {:#x}, protocol_version {}, hostname {}, port {}, next_state {:?}",
    handshake_data.packet_id, handshake_data.protocol_version,
    handshake_data.hostname,handshake_data.port, handshake_data.next_state);

  let packet_data = match handshake_data.next_state {
    HandshakeNextState::Status => match handshake_data.protocol_version {
      version if supports_custom_colors(version) => composed_configs.status_component,
      _ => composed_configs.status,
    }
    HandshakeNextState::Login => match handshake_data.protocol_version {
      version if supports_custom_colors(version) => composed_configs.disconnect_component,
      _ => composed_configs.disconnect,
    }
  };

  // create response packet
  // both disconnect during login (client-bound) & status response share the same packet id: 0x00
  // https://wiki.vg/Protocol#Status_Response
  // https://wiki.vg/Protocol#Disconnect_.28login.29
  let packet = match create_packet(0x00, packet_data) {
    Ok(packet) => packet,
    Err(err) => {
      error!("Unable to encode response: {:?}", err);
      drop(stream);
      return;
    }
  };

  // send response packet to stream & close stream
  match send_flush_close(&packet, &mut stream).await {
    Ok(_) => trace!("Response to {:?} sent successfully", handshake_data.next_state),
    Err(err) => error!("Could not send response: {:?}", err)
  }

  // dropping the stream resource will close the connection
  drop(stream);
  trace!("<- Dropped connection");
}

async fn handle_legacy(n: usize, peek_bytes: &[u8], composed_configs: &ComposedConfigs, mut stream: &mut TcpStream) {
  trace!("Encountered legacy ping!");
  // 1.6            -> FE 01 FA ..  |
  // 1.4 - 1.5      -> FE 01        | handled the same
  // Beta1.8 - 1.3  -> FE
  let post_1_3 = n > 1 && peek_bytes[1] == 0x01;

  let characters = match post_1_3 {
    true => {
      // for 1.4-1.6: separated by 0x0000 -> \u{0000} -> null char
      // - §1: required prefix - 127: protocol version -> recommended (no real legacy version)
      // - {0}: version name   - {1}: motd   - online players   - max players
      let segments: [&str; 6] = ["§1", "127", &composed_configs.status_legacy.1, &composed_configs.status_legacy.0, "0", "0"];
      segments.join("\u{0000}")
    }
    false => {
      // for Beta1.8-1.3: seperated by § -> NO COLOR CODES!!
      // - motd   - online players    - max players
      let segments: [&str; 3] = [&composed_configs.status_legacy.2, "0", "0"];
      segments.join("§")
    }
  };

  let utf16_encoded: Vec<u16> = characters.encode_utf16().collect();

  let mut response_packet = Vec::new();
  // "kick packet" -> 0xFF
  response_packet.write_u8(0xFF).await.expect("Could not write prefix byte to buffer");
  // length of body in characters(not bytes! but shorts:utf16->u16) as short
  response_packet.write_u16(utf16_encoded.len() as u16).await.expect("Could not write u16 to buffer");
  for bin in utf16_encoded {
    response_packet.write_u16(bin).await.expect("Could not write u16 to buffer");
  }

  send_flush_close(&*response_packet, &mut stream).await.expect("TODO: panic message");
}

fn supports_custom_colors(protocol_version: i32) -> bool {
  // protocol version for snapshots after 1.16.4-pre1 are prefixed with 0x40000000, we dont care here
  // 735 -> 1.16: https://wiki.vg/Protocol_version_numbers
  protocol_version >= 735
}

async fn send_flush_close(data: &[u8], stream: &mut TcpStream) -> Result<(), PacketHandleError> {
  stream.write_all(&*data).await?;
  stream.flush().await?;
  stream.shutdown().await?;
  Ok(())
}

fn create_packet(packet_id: i32, content: String) -> Result<Vec<u8>, PacketHandleError> {
  let mut inner = Vec::new();
  VarInt::encode(packet_id, &mut inner)?;
  VarString::encode(content, &mut inner)?;

  let mut outer = Vec::new();
  VarInt::encode(inner.len() as i32, &mut outer)?;
  outer.extend_from_slice(&*inner);

  Ok(outer)
}

fn decode_handshake(mut bytes: &[u8]) -> Result<HandshakeData, PacketHandleError> {
  // https://wiki.vg/Protocol#Handshake
  let packet_id = VarInt::decode(&mut bytes)?;
  let protocol_version = VarInt::decode(&mut bytes)?;
  let hostname = VarString::decode(&mut bytes)?;
  let port = bytes.get_u16(); // u16: short
  let next_state = HandshakeNextState::from(VarInt::decode(&mut bytes)?);

  Ok(HandshakeData { packet_id, protocol_version, hostname, port, next_state })
}

#[derive(Debug)]
struct HandshakeData {
  pub packet_id: i32,
  pub protocol_version: i32,
  pub hostname: String,
  pub port: u16,
  pub next_state: HandshakeNextState,
}

#[repr(i32)]
#[derive(Debug, PartialEq)]
enum HandshakeNextState {
  Status = 1,
  Login = 2,
}

impl From<i32> for HandshakeNextState {
  fn from(value: i32) -> Self {
    unsafe { std::mem::transmute(value) }
  }
}

#[derive(Debug)]
pub enum PacketHandleError {
  InvalidVarInt(VarIntDecodeError),
  InvalidVarString(VarStringDecodeError),
  Io(io::Error),
}

impl From<VarStringDecodeError> for PacketHandleError {
  fn from(err: VarStringDecodeError) -> Self {
    PacketHandleError::InvalidVarString(err)
  }
}

impl From<VarIntDecodeError> for PacketHandleError {
  fn from(err: VarIntDecodeError) -> Self {
    PacketHandleError::InvalidVarInt(err)
  }
}

impl From<io::Error> for PacketHandleError {
  fn from(err: io::Error) -> Self {
    PacketHandleError::Io(err)
  }
}
