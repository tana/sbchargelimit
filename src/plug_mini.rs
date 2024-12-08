use anyhow::{anyhow, Result};
use btleplug::api::{Characteristic, Peripheral as _, WriteType};
use btleplug::platform::Peripheral;
use tokio::sync::mpsc::{self, Receiver};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use uuid::{uuid, Uuid};

const SVC_UUID: Uuid = uuid!("cba20d00-224d-11e6-9fb8-0002a5d5c51b");
const CHR_UUID_RX: Uuid = uuid!("cba20002-224d-11e6-9fb8-0002a5d5c51b");
const CHR_UUID_TX: Uuid = uuid!("cba20003-224d-11e6-9fb8-0002a5d5c51b");
const CMD_EXPANSION: u8 = 0x0F;

pub enum SetStateOperation {
    #[allow(dead_code)]
    TurnOn,
    #[allow(dead_code)]
    TurnOff,
    #[allow(dead_code)]
    Toggle,
}

/// Represents a SwitchBot Plug Mini device
/// Reference: https://github.com/OpenWonderLabs/SwitchBotAPI-BLE/blob/latest/devicetypes/plugmini.md
pub struct PlugMini {
    peripheral: Peripheral,
    tx_chr: Option<Characteristic>,
    rx_chr: Option<Characteristic>,
    notification_task_handle: Option<JoinHandle<()>>,
    chan_receiver: Option<Receiver<Vec<u8>>>,
    initialized: bool,
}

impl PlugMini {
    pub fn new(peripheral: Peripheral) -> Self {
        Self {
            peripheral,
            tx_chr: None,
            rx_chr: None,
            notification_task_handle: None,
            chan_receiver: None,
            initialized: false,
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

        // Response is sent through the TX characteristic
        // (i.e. RX and TX is defined as seen from the device)
        self.peripheral.subscribe(tx_chr).await?;
        let (chan_sender, chan_rx) = mpsc::channel(1);
        let mut notifications = self.peripheral.notifications().await.unwrap();
        self.notification_task_handle = Some(tokio::spawn(async move {
            while let Some(notification) = notifications.next().await {
                if notification.uuid == CHR_UUID_TX {
                    chan_sender.send(notification.value).await.unwrap();
                }
            }
        }));

        self.tx_chr = Some(tx_chr.clone());
        self.rx_chr = Some(rx_chr.clone());
        self.chan_receiver = Some(chan_rx);

        self.initialized = true;

        Ok(())
    }

    pub async fn set_state(&mut self, operation: SetStateOperation) -> Result<bool> {
        let payload = match operation {
            SetStateOperation::TurnOn => [0x50, 0x01, 0x01, 0x80],
            SetStateOperation::TurnOff => [0x50, 0x01, 0x01, 0x00],
            SetStateOperation::Toggle => [0x50, 0x01, 0x02, 0x80],
        };

        let res = self.send_request(CMD_EXPANSION, &payload).await?;

        if res[0] != 0x01 {
            Err(anyhow!("Invalid response"))
        } else {
            match res[1] {
                0x00 => Ok(false),
                0x80 => Ok(true),
                _ => Err(anyhow!("Invalid response")),
            }
        }
    }

    async fn send_request(&mut self, cmd: u8, payload: &[u8]) -> Result<Vec<u8>> {
        if !self.initialized {
            return Err(anyhow!("Device not connected"));
        }

        let mut packet = Vec::with_capacity(2 + payload.len());
        packet.push(0x57); // Magic number
        packet.push((0b00 << 6) | (cmd & 0b1111)); // Header
        packet.extend(payload);

        self.peripheral
            .write(
                self.rx_chr.as_ref().unwrap(),
                &packet,
                WriteType::WithResponse,
            )
            .await?;

        Ok(self
            .chan_receiver
            .as_mut()
            .unwrap()
            .recv()
            .await
            .ok_or(anyhow!("Channel broken"))?)
    }

    pub async fn is_connected(&self) -> Result<bool> {
        Ok(self.peripheral.is_connected().await? && self.initialized)
    }

    pub async fn disconnect(self) -> Result<()> {
        Ok(self.peripheral.disconnect().await?)
    }
}
