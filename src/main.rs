mod config;
mod plug_mini;

use btleplug::api::{BDAddr, Central, CentralEvent, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::Manager;
use config::Config;
use plug_mini::PlugMini;
use tokio_stream::StreamExt;

const APP_NAME: &str = "sbchargelimit";

#[tokio::main]
async fn main() {
    println!(
        "Loading config from {}",
        confy::get_configuration_file_path(APP_NAME, None)
            .unwrap()
            .to_str()
            .unwrap()
    );
    let config: Config = confy::load(APP_NAME, None).unwrap();

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
            if found_peripheral.address() == BDAddr::from_str_delim(&config.plug_mini.addr).unwrap()
            {
                peripheral = Some(found_peripheral);
                break;
            }
        }
    }

    let mut plug = PlugMini::new(peripheral.unwrap());

    plug.connect().await.unwrap();
    println!("Connected");

    plug.set_state(plug_mini::SetStateOperation::Toggle)
        .await
        .unwrap();
}
