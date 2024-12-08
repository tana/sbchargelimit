use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub stop_thresh: f32,
    pub start_thresh: f32,
    pub search_timeout: u64,    // seconds
    pub plug: PlugConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            stop_thresh: 0.6,
            start_thresh: 0.5,
            search_timeout: 10,
            plug: PlugConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct PlugConfig {
    pub addr: String,
}
