use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;

use base64::prelude::Engine as _;
use log::{debug, info, trace, warn};
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

  let config_path = env::var("CONFIG").unwrap_or("config.json".to_string());
  debug!("Loading config from '{}'", config_path);
  let config = load_config(&*config_path);
  debug!("Loaded config {:?}", config);

  let favicon = encode_favicon(&config);
  trace!("Base64 encoded favicon {:?}", favicon);

  // pre compose json responses for less cpu usage (no repetitive encoding)
  let composed_configs = ComposedConfigs::new(favicon, &config);
  debug!("Pre composed json responses successfully");
  trace!("Created composed_configs {:?}", composed_configs);

  let listener = TcpListener::bind(&config.bind).await?;
  info!("Tcp server listening on: {}", listener.local_addr().unwrap());

  loop {
    let (stream, address) = listener.accept().await?;
    debug!("-> Peer connected - {}", address);

    // TODO feat(thread pool): limit & reuse
    tokio::spawn(handshake::handle_client(stream, composed_configs.clone()));
  }
}

fn load_config(config_path: &str) -> Config {
  Config::load(config_path).unwrap_or_else(|err| match err {
    ConfigError::Io(_) => {
      // could not read config file: might not exist -> generate default config
      warn!("There was no config at '{}', created default config", config_path);
      let default_config = Config::default();
      default_config.save(config_path).expect("could not save default config");
      default_config
    }
    ConfigError::Parse(err) => panic!("malformed config, might have changed, delete to regenerate {}", err)
  })
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

#[derive(Debug, Clone)]
pub struct ComposedConfigs {
  status: String,
  status_component: String,
  status_legacy: (String, String, String), // motd, version_text, colorless motd (pre 1.4)
  disconnect: String,
  disconnect_component: String,
}

impl ComposedConfigs {
  fn new(favicon: Option<String>, config: &Config) -> Self {
    let status = ServerStatus::generate_json(favicon.clone(), &config, false);
    let status_component = match config.motd.component {
      Value::Null => status.clone(),
      _ => ServerStatus::generate_json(favicon.clone(), &config, true),
    };
    let status_legacy = (config.motd.legacy.clone(), config.version.text.clone(), ComposedConfigs::strip_color_codes(&config.motd.legacy));

    let disconnect = DisconnectMessage::generate_json(&config, false);
    let disconnect_component = match config.disconnect.component {
      Value::Null => disconnect.clone(),
      _ => DisconnectMessage::generate_json(&config, true),
    };

    Self { status, status_component, status_legacy, disconnect, disconnect_component }
  }

  // ensures no § is present
  fn strip_color_codes(text: &String) -> String {
    let mut result = String::new();
    let mut skip = false;

    for c in text.chars() {
      if skip || c == '§' {
        skip = c == '§';
        continue;
      }
      result.push(c);
    }

    result
  }
}
