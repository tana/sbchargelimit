#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod config;
mod plug_mini;
mod tray_icon;

use std::time::Duration;

use anyhow::{anyhow, Result};
use btleplug::api::{BDAddr, Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use config::Config;
use plug_mini::{PlugMini, SetStateOperation};
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

    let actual_main_handle = tokio::spawn(actual_main(config));

    // Run run_tray_icon_loop in main thread
    // (because sometimes GUI does not correctly work on other threads)
    tokio::task::block_in_place(tray_icon::run_tray_icon_loop);

    actual_main_handle.await.unwrap(); // join
}

async fn actual_main(config: Config) {
    // Initialize battery state access
    let battery_manager = starship_battery::Manager::new().unwrap();
    // Use first battery
    let mut battery = battery_manager
        .batteries()
        .unwrap()
        .next()
        .ok_or(anyhow!("No battery found"))
        .unwrap()
        .unwrap();

    // Init BLE central
    let manager = Manager::new().await.unwrap();
    let mut central = manager
        .adapters()
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    let mut plug = connect_plug(&mut central, &config).await.unwrap();

    // Periodically do operations at a constant interval
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;

        battery_manager.refresh(&mut battery).unwrap();

        let remaining = battery.state_of_charge().value;
        let state = battery.state();

        println!("{:?} {}", state, remaining);
        match state {
            starship_battery::State::Charging | starship_battery::State::Full
                if remaining > config.stop_thresh =>
            {
                println!("TurnOff");

                // Reconnect if needded
                if !plug.is_connected().await.unwrap() {
                    plug = connect_plug(&mut central, &config).await.unwrap();
                }
                plug.set_state(SetStateOperation::TurnOff).await.unwrap();
            }
            _ if remaining < config.start_thresh => {
                println!("TurnOn");

                // Reconnect if needded
                if !plug.is_connected().await.unwrap() {
                    plug = connect_plug(&mut central, &config).await.unwrap();
                }
                plug.set_state(SetStateOperation::TurnOn).await.unwrap();
            }
            _ => (),
        }
    }
}

async fn connect_plug(central: &mut Adapter, config: &Config) -> Result<PlugMini> {
    let peripheral = tokio::time::timeout(
        Duration::from_secs(config.search_timeout),
        search_plug(central, config),
    )
    .await??;
    let mut plug = PlugMini::new(peripheral);

    plug.connect().await?;
    println!("Connected");

    Ok(plug)
}

// Search for a SwitchBot Plug Mini
async fn search_plug(central: &mut Adapter, config: &Config) -> Result<Peripheral> {
    println!("Searching for the device...");
    central.start_scan(ScanFilter::default()).await?;
    let mut events = central.events().await?;
    while let Some(evt) = events.next().await {
        if let CentralEvent::DeviceDiscovered(id) = evt {
            let found_peripheral = central.peripheral(&id).await?;
            // Use the first device which matches with the specified MAC address
            if found_peripheral.address() == BDAddr::from_str_delim(&config.plug_mini.addr)? {
                return Ok(found_peripheral);
            }
        }
    }

    unreachable!()
}
