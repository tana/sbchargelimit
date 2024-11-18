use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub stop_thresh: f32,
    pub start_thresh: f32,
    pub device: Vec<DeviceConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            stop_thresh: 0.6,
            start_thresh: 0.5,
            device: vec![DeviceConfig::PlugMini(PlugMiniConfig {
                addr: String::from(""),
                search_timeout: default_search_timeout(),
            })],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlugMiniConfig {
    pub addr: String,
    #[serde(default = "default_search_timeout")]
    pub search_timeout: u64, // seconds
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TypeCSwitchConfig {
    pub addr: String,
    #[serde(default = "default_search_timeout")]
    pub search_timeout: u64, // seconds
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum DeviceConfig {
    PlugMini(PlugMiniConfig),
    TypeCSwitch(TypeCSwitchConfig),
}

fn default_search_timeout() -> u64 {
    10
}
