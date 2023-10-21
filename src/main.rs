use std::error::Error;
use std::fs::File;
use std::io::Read;

use base64::prelude::Engine as _;
use log::{debug, info, warn};
use serde_json::Value;
use tokio::net::TcpListener;

use crate::config::{Config, ConfigError, DisconnectMessage, ServerStatus};

mod var_int;
mod config;
mod handshake;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
      .init();

  // TODO feat(config): read runtime args for config path
  let config_path = "config.json";
  debug!("Loading config from '{}'", config_path);

  let config = load_config(config_path);
  debug!("Loaded config {:?}", config);

  let favicon = encode_favicon(&config);
  debug!("Base64 encoded favicon {:?}", favicon);

  // pre compose json responses for less cpu usage
  let server_status = ServerStatus::generate_json(favicon.clone(), &config, false);
  let server_status_component = if config.motd.component == Value::Null { server_status.clone() } else { ServerStatus::generate_json(favicon.clone(), &config, true) };
  drop(favicon);
  let disconnect = DisconnectMessage::generate_json(&config);
  debug!("Pre composed json responses successfully");


  let listener = TcpListener::bind(&config.bind).await?;
  info!("Tcp server listening on: {}", listener.local_addr().unwrap());

  loop {
    let (stream, address) = listener.accept().await?;
    debug!("-> Peer connected - {}", address);

    // TODO feat(thread pool): limit & reuse
    tokio::spawn(handshake::handle_client(stream, server_status_component.clone(), disconnect.clone()));
  }
}

fn load_config(config_path: &str) -> Config {
  match Config::load(config_path) {
    Ok(config) => config,
    Err(err) => match err {
      ConfigError::Io(_) => {
        // could not read config file: might not exist -> generate default config
        warn!("There was no config at '{}', created default config", config_path);
        let default_config = Config::default();
        default_config.save(config_path).expect("could not save default config");
        default_config
      }
      ConfigError::Parse(err) => panic!("malformed config, might have changed, delete to regenerate {}", err)
    }
  }
}

fn encode_favicon(config: &Config) -> Option<String> {
  match &config.favicon {
    Some(favicon) => match File::open(favicon) {
      Ok(mut file) => {
        // TODO also check dimensions
        let mut content = Vec::new();
        file.read_to_end(&mut content).expect("error while reading favicon");
        Some(base64::engine::general_purpose::STANDARD_NO_PAD.encode(&mut content))
      }
      Err(err) => {
        warn!("Could not find specified icon {:?}", err.to_string());
        None
      }
    }
    None => None,
  }
}
