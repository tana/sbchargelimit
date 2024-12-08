#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod config;
mod tray_icon;

use std::fs::{self, OpenOptions};
use std::time::Duration;

use anyhow::{anyhow, Result};
use btleplug::api::{BDAddr, Manager as _, Peripheral as _};
use btleplug::platform::{Adapter, Manager};
use config::Config;
use directories::ProjectDirs;
use env_logger::Env;
use pluglib::{SmartPlug, SmartPlugEnum};

const APP_NAME: &str = "sbchargelimit";

#[tokio::main]
async fn main() {
    let mut logger_builder =
        env_logger::Builder::from_env(Env::default().default_filter_or("info"));
    // Same as in `confy`
    let log_path = match ProjectDirs::from("rs", "", APP_NAME) {
        Some(project_dirs) => {
            let now = chrono::Local::now();
            let target = project_dirs
                .cache_dir()
                .join(now.format("log_%Y%m%d_%H%M%S.log").to_string());

            fs::create_dir_all(target.parent().unwrap()).unwrap();

            logger_builder.target(env_logger::Target::Pipe(Box::new(
                OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&target)
                    .unwrap(),
            )));

            Some(target)
        }
        None => None,
    };
    logger_builder.init();

    log::info!(
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
    tokio::task::block_in_place(|| tray_icon::run_tray_icon_loop(log_path));

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

    let mut plug = Some(connect_plug(&mut central, &config).await.unwrap());

    // Periodically do operations at a constant interval
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;

        battery_manager.refresh(&mut battery).unwrap();

        let remaining = battery.state_of_charge().value;
        let state = battery.state();

        if plug.is_some() {
            if !plug.as_ref().unwrap().peripheral().is_connected().await.unwrap() {
                plug.take().unwrap().peripheral().disconnect().await.unwrap();
            }
        }

        // Reconnect if needded
        if plug.is_none() {
            match connect_plug(&mut central, &config).await {
                Ok(p) => plug = Some(p),
                Err(e) => {
                    log::error!("{}", e);
                    continue;
                }
            }
        }

        log::debug!("{:?} {}", state, remaining);
        match state {
            starship_battery::State::Charging | starship_battery::State::Full
                if remaining > config.stop_thresh =>
            {
                log::info!("TurnOff");

                if let Some(ref mut plug) = plug {
                    plug.set_state(false).await.unwrap();
                }
            }
            starship_battery::State::Discharging
            | starship_battery::State::Empty
            | starship_battery::State::Unknown
                if remaining < config.start_thresh =>
            {
                log::info!("TurnOn");

                if let Some(ref mut plug) = plug {
                    plug.set_state(true).await.unwrap();
                }
            }
            _ => (),
        }
    }
}

async fn connect_plug(central: &mut Adapter, config: &Config) -> Result<SmartPlugEnum> {
    let addr = BDAddr::from_str_delim(&config.plug.addr)?;

    let plug = tokio::time::timeout(
        Duration::from_secs(config.search_timeout),
        pluglib::scan_and_connect(&central, |peripheral| peripheral.address() == addr),
    )
    .await??;

    log::info!("Connected");

    Ok(plug)
}
