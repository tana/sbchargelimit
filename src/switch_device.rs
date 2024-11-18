use anyhow::Result;

pub mod plug_mini;
pub mod type_c_switch;

#[async_trait::async_trait]
pub trait SwitchDevice: Send {
    async fn set_on_off(&mut self, state: bool) -> Result<()>;
    async fn connect(&mut self) -> Result<()>;
    async fn is_connected(&self) -> Result<bool>;
    async fn disconnect(&mut self) -> Result<()>;
}
