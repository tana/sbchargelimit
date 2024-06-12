use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Config {
    pub plug_mini: PlugMiniConfig,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct PlugMiniConfig {
    pub addr: String,
}
