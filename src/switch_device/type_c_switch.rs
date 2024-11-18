//! Driver for DIY Type-C switch device

use std::time::Duration;

use anyhow::{anyhow, Result};
use btleplug::{
    api::{BDAddr, Central as _, CentralEvent, Characteristic, Peripheral as _, ScanFilter, WriteType},
    platform::{Adapter, Peripheral},
};
use tokio_stream::StreamExt as _;
use uuid::{uuid, Uuid};

use crate::config::TypeCSwitchConfig;

use super::SwitchDevice;

const SVC_UUID: Uuid = uuid!("a6302557-c9ae-44b2-b987-73072a0e6d84");
const CHR_UUID_STATE: Uuid = uuid!("e612f461-7a74-4946-bc25-03968fbdecda");

pub struct TypeCSwitch {
    peripheral: Peripheral,
    state_chr: Option<Characteristic>,
    initialized: bool,
}

impl TypeCSwitch {
    pub fn new(peripheral: Peripheral) -> Self {
        Self {
            peripheral,
            state_chr: None,
            initialized: false,
        }
    }

    async fn search_inner(central: &mut Adapter, config: &TypeCSwitchConfig) -> Result<Peripheral> {
        log::info!("Searching for the device...");
        central.start_scan(ScanFilter::default()).await?;
        let mut events = central.events().await?;
        while let Some(evt) = events.next().await {
            if let CentralEvent::DeviceDiscovered(id) = evt {
                let found_peripheral = central.peripheral(&id).await?;
                // Use the first device which matches with the specified MAC address
                if found_peripheral.address() == BDAddr::from_str_delim(&config.addr)? {
                    central.stop_scan().await?;
                    return Ok(found_peripheral);
                }
            }
        }

        unreachable!()
    }

    // Search for a device
    pub async fn search(central: &mut Adapter, config: &TypeCSwitchConfig) -> Result<Peripheral> {
        Ok(tokio::time::timeout(
            Duration::from_secs(config.search_timeout),
            Self::search_inner(central, config),
        )
        .await??)
    }
}

#[async_trait::async_trait]
impl SwitchDevice for TypeCSwitch {
    async fn set_on_off(&mut self, state: bool) -> Result<()> {
        if !self.initialized {
            return Err(anyhow!("Device not connected"));
        }

        self.peripheral
            .write(
                self.state_chr.as_ref().unwrap(),
                &[if state { 1 } else { 0 }],
                WriteType::WithResponse,
            )
            .await?;

        Ok(())
    }

    async fn connect(&mut self) -> Result<()> {
        self.peripheral.connect().await?;

        self.peripheral.discover_services().await?;

        let services = self.peripheral.services();

        let Some(service) = services.iter().filter(|s| s.uuid == SVC_UUID).next() else {
            return Err(anyhow!("Type C switch service not found"));
        };

        let Some(state_chr) = service
            .characteristics
            .iter()
            .filter(|c| c.uuid == CHR_UUID_STATE)
            .next()
        else {
            return Err(anyhow!("State characteristic not found"));
        };

        self.state_chr = Some(state_chr.clone());

        self.initialized = true;

        Ok(())
    }

    async fn is_connected(&self) -> Result<bool> {
        Ok(self.peripheral.is_connected().await? && self.initialized)
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.peripheral.disconnect().await?;
        Ok(())
    }
}
