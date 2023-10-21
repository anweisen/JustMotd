use std::error::Error;
use std::fmt::Write;
use std::fs::File;
use std::io::Read;

use base64::prelude::Engine as _;
use bytes::{Buf, BufMut};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::config::{Config, ConfigError, ServerStatus};
use crate::var_int::{VarInt, VarIntDecodeError, VarString, VarStringDecodeError};

mod var_int;
mod packet;
mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let config_path = "config.json"; // TODO feat(config): read runtime args for config path
  let config = load_config(config_path);
  println!("{:?}", config);

  let favicon = decode_favicon(&config);

  let server_status = ServerStatus::generate_json(favicon.clone(), &config, false);
  let server_status_component = if config.motd.component == Value::Null { server_status.clone() } else { ServerStatus::generate_json(favicon.clone(), &config, true) };
  println!("{:?}", server_status);
  drop(favicon);

  let listener = TcpListener::bind(&config.bind).await?;
  println!("Listening on tcp: {}", listener.local_addr().unwrap());

  loop {
    let (stream, _) = listener.accept().await?;
    tokio::spawn(handle_client(stream, server_status.clone())); // TODO feat(thread pool): limit & reuse
  }
}

fn load_config(config_path: &str) -> Config {
  match Config::load(config_path) {
    Ok(config) => config,
    Err(err) => match err {
      ConfigError::Io(_) => {
        // could not read config file: might not exist -> generate default config
        // TODO log
        let default_config = Config::default();
        default_config.save(config_path).expect("could not save default config");
        default_config
      }
      ConfigError::Parse(_err) => panic!("malformed config, might have changed, delete to regenerate")
    }
  }
}

fn decode_favicon(config: &Config) -> Option<String> {
  match &config.favicon {
    Some(favicon) => match File::open(favicon) {
      Ok(mut file) => {
        // TODO also check dimensions
        let mut content = Vec::new();
        file.read_to_end(&mut content).expect("error while reading favicon");
        println!("{}", base64::engine::general_purpose::STANDARD_NO_PAD.encode(&mut content));
        Some(base64::engine::general_purpose::STANDARD_NO_PAD.encode(&mut content))
      }
      Err(err) => {
        // TODO log there is no favicon
        println!("Could not find specified icon at {:?}", err.to_string());
        None
      }
    }
    None => None,
  }
}

async fn handle_client(mut stream: TcpStream, server_status: String) {
  let length = match VarInt::decode_partial(&mut stream).await {
    Ok(length) => length,
    Err(VarIntDecodeError::Incomplete) => return,
    Err(VarIntDecodeError::TooLarge) => panic!("")
  };

  // TODO feat(legacy ping): https://wiki.vg/Server_List_Ping#1.6
  if length == 0xFE {
    return;
  }

  println!();
  println!("Received Packet!");
  println!("Length: {}", length);


  let mut bytes = vec![0u8; length as usize];
  stream.read_exact(&mut bytes).await.expect("TODO: panic message");

  // let mut buffer = BytesMut::with_capacity(length as usize);
  // buffer.extend_from_slice(&*bytes);

  let x = read_handshake(&mut bytes).expect("TODO: panic message");

  let mut res = Vec::new();
  VarInt::encode(0x00, &mut res).expect("TODO: panic message");
  VarString::encode(server_status, &mut res).expect("TODO: panic message");

  let mut packet = Vec::new();
  VarInt::encode(res.len() as i32, &mut packet).expect("TODO: panic message");
  packet.write_all(&*res).await.expect("TODO: panic message");

  println!("{:?}", packet);
  let mut copy = &packet[..];
  println!("length {:?}", VarInt::decode(&mut copy));
  println!("packet id {:?}", VarInt::decode(&mut copy));
  // println!("text {:?}", VarString::decode(&mut copy));

  stream.write_all(&*packet).await.expect("TODO: panic message");
  stream.flush().await.expect("TODO: panic message");
  println!("msg sent");

  // dropping the stream resource will close the connection
  drop(stream);
  println!("Closed Connection");
}

#[derive(Debug)]
pub enum PacketHandleError {
  InvalidVarInt,
  InvalidVarString,
  IoError,
}

impl From<VarStringDecodeError> for PacketHandleError {
  fn from(value: VarStringDecodeError) -> Self {
    match value {
      VarStringDecodeError::InvalidVarInt => PacketHandleError::InvalidVarInt,
      VarStringDecodeError::UtfError => PacketHandleError::InvalidVarString,
    }
  }
}

impl From<VarIntDecodeError> for PacketHandleError {
  fn from(_value: VarIntDecodeError) -> Self {
    PacketHandleError::InvalidVarInt
  }
}

fn read_handshake(mut bytes: &[u8]) -> Result<i32, PacketHandleError> {
  // https://wiki.vg/Protocol#Handshake
  let packet_id = VarInt::decode(&mut bytes)?;
  let protocol_version = VarInt::decode(&mut bytes)?;
  let hostname = VarString::decode(&mut bytes)?;
  let port = bytes.get_u16(); // u16: short
  let next_state = VarInt::decode(&mut bytes)?;

  println!("Packet ID {}", packet_id);
  println!("Protocol Version {}", protocol_version);
  println!("Hostname {}", hostname);
  println!("Port {}", port);
  println!("State {}", next_state);

  // TODO refactor(state enum)
  if next_state == 1 {}

  Ok(protocol_version)
}
