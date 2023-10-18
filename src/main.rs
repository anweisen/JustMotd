use std::error::Error;
use std::fmt::Write;
use std::io::Read;

use bytes::{Buf, BufMut, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
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




  let json = r#"
    {
      "version": {
        "name": "test version",
        "protocol": 762
      },
      "players": {
        "max": 100,
        "online": 5,
        "sample": [
          {
            "name": "anweisen",
            "id": "4566e69f-c907-48ee-8d71-d7ba5aa00d20"
          }
        ]
      },
      "description": {
        "text": "Hello World!"
      },
      "enforcesSecureChat": true,
      "previewsChat": true
    }
  "#.replace(" ", "").replace("\n", "");

  // let mut res = BytesMut::with_capacity(128);
  let mut res = Vec::new();
  VarInt::encode(0x00, &mut res).expect("TODO: panic message");
  VarString::encode(json, &mut res).expect("");

  let mut packet = Vec::new();
  VarInt::encode(res.len() as i32, &mut packet).expect("TODO: panic message");
  packet.write_all(&*res).await.expect("TODO: panic message");

  stream.write_all(&*packet).await.expect("");
  stream.flush().await.expect("");
  println!("msg sent");
}

fn read_handshake(mut bytes: &[u8]) -> Result<(), VarIntDecodeError> {
  // https://wiki.vg/Protocol#Handshake
  let packet_id = VarInt::decode(&mut bytes)?;
  let protocol_version = VarInt::decode(&mut bytes)?;
  let hostname = VarString::decode(&mut bytes);
  let port = bytes.get_u16(); // u16: short
  let next_state = VarInt::decode(&mut bytes)?;

  println!("Packet ID {}", packet_id);
  println!("Protocol Version {}", protocol_version);
  println!("Hostname {}", hostname);
  println!("Port {}", port);
  println!("State {}", next_state);

  // TODO refactor(state enum)
  if next_state == 1 {}

  Ok(())
}
