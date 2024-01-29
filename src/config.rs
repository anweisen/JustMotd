use std::fs;
use std::fs::File;
use std::io::Write;
use log::warn;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
  pub bind: String,
  pub favicon: Option<String>,
  pub motd: MotdConfig,
  pub version: VersionConfig,
  pub disconnect: DisconnectConfig,

  #[serde(flatten)]
  unknown_fields: std::collections::BTreeMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MotdConfig {
  pub text: String,
  pub legacy: String,
  pub component: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VersionConfig {
  pub text: String,
  pub hover: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DisconnectConfig {
  pub text: String,
  pub component: Value,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      bind: "0.0.0.0:25565".to_string(),
      favicon: Some("icon.png".to_string()),
      motd: MotdConfig {
        text: "§cServer is currently unreachable\n§8› §7§ogithub.com/anweisen/§lJustMotd".to_string(),
        legacy: "§cpowered by JustMotd".to_string(),
        component: Value::Null,
      },
      version: VersionConfig {
        text: "§4§l✗ §cOffline ".to_string(),
        hover: vec![" ".to_string(), "  §8× §canweisen.net §8×  ".to_string(), "  ".to_string()],
      },
      disconnect: DisconnectConfig {
        text: "§cThis server is currently undergoing maintenance".to_string(),
        component: Value::Null,
      },
      unknown_fields: Default::default(),
    }
  }
}

impl Config {
  pub fn load(path: &str) -> Result<Config, ConfigError> {
    let raw = fs::read_to_string(path).map_err(|err| ConfigError::Io(err))?;
    let config: Self = serde_json::from_str(&*raw).map_err(|err| ConfigError::Parse(err))?;

    for field in &config.unknown_fields {
      warn!("Unknown configuration '{}' with value {:?}", field.0, field.1);
    }

    Ok(config)
  }

  pub fn save(&self, path: &str) -> Result<(), ConfigError> {
    let raw = serde_json::to_string_pretty(self).map_err(|err| ConfigError::Parse(err))?;
    let mut file = File::create(path).map_err(|err| ConfigError::Io(err))?;
    file.write_all(raw.as_bytes()).map_err(|err| ConfigError::Io(err))
  }
}

#[derive(Debug)]
pub enum ConfigError {
  Io(std::io::Error),
  Parse(serde_json::Error),
}


#[derive(Serialize, Deserialize, Debug)]
pub struct ServerStatus {
  #[serde(rename = "enforcesSecureChat")]
  enforces_secure_chat: bool,
  #[serde(rename = "previewsChat")]
  previews_chat: bool,
  version: ServerStatusVersion,
  players: ServerStatusPlayers,
  description: Value,
  favicon: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerStatusVersion {
  name: String,
  protocol: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerStatusPlayers {
  max: i32,
  online: i32,
  sample: Vec<ServerStatusSamplePlayer>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerStatusSamplePlayer {
  name: String,
  id: String, // uuid
}

static UUID: &str = "147e3454-1727-4807-9ba5-fe35b25ddbc1";

impl ServerStatus {
  pub fn generate_json(favicon_base64: Option<String>, config: &Config, use_motd_component: bool) -> String {
    match serde_json::to_string(&ServerStatus {
      favicon: favicon_base64.map(|str| "data:image/png;base64,".to_string() + &*str),
      enforces_secure_chat: false,
      previews_chat: false,
      version: ServerStatusVersion {
        name: config.version.text.clone(),
        protocol: -1,
      },
      players: ServerStatusPlayers {
        // (max) player count must match sample size for some reason to be displayed
        max: config.version.hover.len() as i32,
        online: config.version.hover.len() as i32,
        sample: config.version.hover.iter().map(|name| ServerStatusSamplePlayer { name: name.to_string(), id: UUID.to_string() }).collect(),
      },
      description: if use_motd_component { config.motd.component.clone() } else { json!({"text": &config.motd.text}) },
    }) {
      Ok(json) => json,
      Err(_) => panic!("error while converting generated motd json to string")
    }
  }
}

pub struct DisconnectMessage;

impl DisconnectMessage {
  pub fn generate_json(config: &Config, use_disconnect_component: bool) -> String {
    if use_disconnect_component { config.disconnect.component.to_string() } else { json!({"text": config.disconnect.text}).to_string() }
  }
}
