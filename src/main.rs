use std::error::Error;
use std::io::Read;

use bytes::{Buf, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::net::{TcpListener, TcpStream};

use crate::var_int::{VarInt, VarIntDecodeError, VarString};

mod var_int;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let listener = TcpListener::bind("127.0.0.1:25565").await?;
  println!("Listening on tcp: {}", listener.local_addr().unwrap());

  loop {
    let (stream, addr) = listener.accept().await?;
    tokio::spawn(handle_client(stream)); // TODO feat(thread pool): limit & reuse
  }
}

async fn handle_client(mut stream: TcpStream) {
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
  // for x in &bytes {
  //   println!("{:?}  {:b}  {:#x}", x, x, x);
  // }

  let mut buffer = BytesMut::with_capacity(length as usize);
  buffer.extend_from_slice(&*bytes);

  read_handshake(&mut buffer).expect("TODO: panic message");
}

fn read_handshake(mut bytes: &[u8]) -> Result<(), VarIntDecodeError> {
  let packet_id = VarInt::decode(&mut bytes)?;
  let protocol_version = VarInt::decode(&mut bytes)?;
  let hostname = VarString::decode(&mut bytes);
  let port = bytes.get_u16();
  let state = VarInt::decode(&mut bytes)?;

  println!("Packet ID {}", packet_id);
  println!("Protocol Version {}", protocol_version);
  println!("Hostname {}", hostname);
  println!("Port {}", port);
  println!("State {}", state);

  Ok(())
}
