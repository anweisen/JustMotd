use std::io;

use bytes::Buf;
use log::{error, trace};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::ComposedConfigs;
use crate::var_int::{VarInt, VarIntDecodeError, VarString, VarStringDecodeError};

pub async fn handle_client(mut stream: TcpStream, composed_configs: ComposedConfigs) {
  let length = match VarInt::decode_partial(&mut stream).await {
    Ok(length) => length,
    Err(VarIntDecodeError::Incomplete) => return,
    Err(VarIntDecodeError::TooLarge) => {
      error!("Unable to decode VarInt: TooLarge");
      return;
    }
  };

  // TODO feat(legacy ping): https://wiki.vg/Server_List_Ping#1.6
  if length == 0xFE { // could also be packet with length 254=0xFE (unlikely, but possible)
    trace!("Encountered legacy ping!");
    drop(stream);
    return;
  }

  trace!("Received packet prefixed with (length) {}", length);

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
      println!("Unable to decode handshake: {:?}", err);
      return;
    }
  };

  trace!("Decoded: packet_id {:#x}, protocol_version {}, hostname {}, port {}, next_state {:?}", handshake_data.0, handshake_data.1, handshake_data.2, handshake_data.3, handshake_data.4);

  let packet_data = match handshake_data.4 {
    HandshakeNextState::Status => match handshake_data.1 {
      version if supports_custom_colors(version) => composed_configs.motd_component,
      _ => composed_configs.motd,
    }
    HandshakeNextState::Login => match handshake_data.1 {
      version if supports_custom_colors(version) => composed_configs.disconnect_component,
      _ => composed_configs.disconnect,
    }
  };

  // create response packet, both disconnect during login (client-bound) & status response share the same packet id 0x00
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

  // send response packet to stream
  stream.write_all(&*packet).await.expect("TODO: panic message");
  stream.flush().await.expect("TODO: panic message");
  trace!("Response to {:?} sent successfully", handshake_data.4);

  // dropping the stream resource will close the connection
  drop(stream);
  trace!("Dropped connection");
}

fn supports_custom_colors(protocol_version: i32) -> bool {
  // protocol version for snapshots after 1.16.4-pre1 are prefixed with 0x40000000, we dont care here
  // 735 -> 1.16: https://wiki.vg/Protocol_version_numbers
  protocol_version >= 735
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
  let next_state = VarInt::decode(&mut bytes)?;

  Ok(HandshakeData(packet_id, protocol_version, hostname, port, unsafe { std::mem::transmute(next_state) }))
}

#[derive(Debug)]
struct HandshakeData(i32, i32, String, u16, HandshakeNextState);

#[repr(i32)]
#[derive(Debug, PartialEq)]
enum HandshakeNextState {
  Status = 1,
  Login = 2,
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
