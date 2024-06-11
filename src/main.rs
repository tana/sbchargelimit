mod plug_mini;

use btleplug::api::{BDAddr, Central, CentralEvent, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::Manager;
use plug_mini::PlugMini;
use tokio_stream::StreamExt;

const MAC_ADDR: &str = env!("SWITCHBOT_PLUG_ADDR");

#[tokio::main]
async fn main() {
    // Init BLE central
    let manager = Manager::new().await.unwrap();
    let central = manager
        .adapters()
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    // Search a SwitchBot Plug Mini
    println!("Searching for the device...");
    central.start_scan(ScanFilter::default()).await.unwrap();
    let mut events = central.events().await.unwrap();
    let mut peripheral = None;
    while let Some(evt) = events.next().await {
        if let CentralEvent::DeviceDiscovered(id) = evt {
            let found_peripheral = central.peripheral(&id).await.unwrap();
            // Use the first device which matches with the specified MAC address
            if found_peripheral.address() == BDAddr::from_str_delim(MAC_ADDR).unwrap() {
                peripheral = Some(found_peripheral);
                break;
            }
        }
    }

    let mut plug = PlugMini::new(peripheral.unwrap());

    plug.connect().await.unwrap();
}
