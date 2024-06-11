use anyhow::{anyhow, Result};
use btleplug::api::{Characteristic, Peripheral as _, WriteType};
use btleplug::platform::Peripheral;
use uuid::{uuid, Uuid};

const SVC_UUID: Uuid = uuid!("cba20d00-224d-11e6-9fb8-0002a5d5c51b");
const CHR_UUID_RX: Uuid = uuid!("cba20002-224d-11e6-9fb8-0002a5d5c51b");
const CHR_UUID_TX: Uuid = uuid!("cba20003-224d-11e6-9fb8-0002a5d5c51b");
const CMD_EXPANSION: u8 = 0x0F;

/// Represents a SwitchBot Plug Mini device
/// Reference: https://github.com/OpenWonderLabs/SwitchBotAPI-BLE/blob/latest/devicetypes/plugmini.md
pub struct PlugMini {
    peripheral: Peripheral,
    tx_chr: Option<Characteristic>,
    rx_chr: Option<Characteristic>,
    connected: bool,
}

impl PlugMini {
    pub fn new(peripheral: Peripheral) -> Self {
        Self {
            peripheral,
            tx_chr: None,
            rx_chr: None,
            connected: false,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        self.peripheral.connect().await?;

        self.peripheral.discover_services().await?;

        let services = self.peripheral.services();

        let Some(service) = services.iter().filter(|s| s.uuid == SVC_UUID).next() else {
            return Err(anyhow!("Plug Mini service not found"));
        };

        let Some(tx_chr) = service
            .characteristics
            .iter()
            .filter(|c| c.uuid == CHR_UUID_TX)
            .next()
        else {
            return Err(anyhow!("TX characteristic not found"));
        };
        let Some(rx_chr) = service
            .characteristics
            .iter()
            .filter(|c| c.uuid == CHR_UUID_RX)
            .next()
        else {
            return Err(anyhow!("RX characteristic not found"));
        };

        self.peripheral.subscribe(rx_chr).await?;

        self.tx_chr = Some(tx_chr.clone());
        self.rx_chr = Some(rx_chr.clone());

        self.connected = true;

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        self.peripheral.disconnect().await?;

        Ok(())
    }

    async fn send_request(&mut self, cmd: u8, payload: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(anyhow!("Device not connected"));
        }

        let mut packet = Vec::with_capacity(2 + payload.len());
        packet.push(0x57); // Magic number
        packet.push((0b00 << 6) | (cmd & 0b111)); // Header
        packet.extend(payload);

        self.peripheral
            .write(
                self.tx_chr.as_ref().unwrap(),
                &packet,
                WriteType::WithResponse,
            )
            .await?;

        Ok(())
    }
}
