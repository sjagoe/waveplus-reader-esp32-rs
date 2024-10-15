use anyhow::{bail, Result};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::prelude::Peripherals,
    // sys::esp_restart,
    wifi::{AuthMethod, WifiEvent},
};
use log::*;
use time::PrimitiveDateTime;

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::netif::IpEvent;
use esp_idf_svc::sntp::{EspSntp, SntpConf, SyncStatus};
use esp_idf_svc::sys::{esp, esp_wifi_connect};

mod ble;
mod http;
mod measurement;
mod rgbled;
mod time;
mod wifi;

use ble::{get_waveplus, read_waveplus};
use http::send_measurement;
use rgbled::{RGB8, WS2812RMT};
use time::get_datetime;
use wifi::{connect_wifi, wait_for_connected};

/// This configuration is picked up at compile time by `build.rs` from the
/// file `cfg.toml`.
#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
    #[default("")]
    waveplus_serial: &'static str,
    #[default(30)]
    read_interval: u16,
    #[default("")]
    server: &'static str,
    #[default("pool.ntp.org")]
    ntp_server: &'static str,
}

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let app_config = CONFIG;

    let peripherals = Peripherals::take().unwrap();

    // Start the LED off yellow
    let mut led = WS2812RMT::new(peripherals.pins.gpio8, peripherals.rmt.channel0)?;
    led.set_pixel(RGB8::new(50, 50, 0))?;

    let sysloop = EspSystemEventLoop::take()?;

    if app_config.wifi_ssid.is_empty() {
        bail!("Missing WiFi name")
    }

    info!("SSID: {:?}", app_config.wifi_ssid);

    let mut wifi = connect_wifi(
        peripherals.modem,
        sysloop.clone(),
        None,
        AuthMethod::WPA2Personal,
        app_config.wifi_ssid,
        app_config.wifi_psk,
    )?;

    info!("Subscribing to events");
    let _wifi_event_sub = sysloop.subscribe::<WifiEvent, _>(move |event| match event {
        WifiEvent::StaDisconnected => {
            error!("Received STA Disconnected event {:?}", event);
            FreeRtos::delay_ms(1000);
            // NOTE: calling the FFI binding directly to prevent casusing a move
            // on the the EspWifi instance.
            if let Err(err) = esp!(unsafe { esp_wifi_connect() }) {
                info!("Error calling wifi.connect in wifi reconnect {:?}", err);
            }
        }
        _ => info!("Received other Wifi event: {:?}", event),
    })?;

    let _ip_event_sub = sysloop.subscribe::<IpEvent, _>(move |event| {
        info!("Received other IPEvent: {:?}", event);
    });

    info!("Initializing wifi");
    wait_for_connected(&wifi)?;

    // SNTP

    let sntp_conf = SntpConf::<'_> {
        servers: [app_config.ntp_server],
        ..Default::default()
    };
    let sntp = EspSntp::new(&sntp_conf)?;

    wait_for_sntp(&sntp);

    led.set_pixel(RGB8::new(0, 50, 0))?;

    let serial: u32 = app_config.waveplus_serial.parse()?;
    let waveplus = get_waveplus(&serial).expect("Unable to get waveplus bt device");
    let mut state: State = State::Run;
    let mut last_run: Option<PrimitiveDateTime> = None;
    loop {
        info!("Current state: {:?}", state);
        match state {
            State::WifiReconnect => {
                led.set_pixel(RGB8::new(50, 0, 0))?;

                warn!(
                    "Wifi connected: {:?}, up: {:?}",
                    wifi.is_connected()?,
                    wifi.is_up()?
                );

                if wifi.is_connected()? {
                    // If we're here and the wifi device thinks it's
                    // connected, trigger a disconnect event and wait
                    // for re-connect.
                    warn!("Disconnecting wifi for connection retry");
                    if let Err(err) = wifi.disconnect() {
                        error!("Error calling wifi.disconnect after http failure {:?}", err);
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(250));

                led.set_pixel(RGB8::new(50, 50, 0))?;
                warn!(
                    "Wifi connected: {:?}, up: {:?}",
                    wifi.is_connected()?,
                    wifi.is_up()?
                );

                wait_for_connected(&wifi)?;

                led.set_pixel(RGB8::new(0, 50, 0))?;

                state = State::Run;
            }
            State::Run => {
                led.set_pixel(RGB8::new(0, 0, 50))?;

                let current = get_datetime()?;
                let include_radon = should_include_radon(last_run, current);
                warn!("Include radon measurement? {:?}", include_radon);
                let measurement = read_waveplus(&serial, &waveplus, include_radon)?;
                last_run = Some(current);
                led.set_pixel(RGB8::new(50, 50, 0))?;
                let next_state = if send_measurement(app_config.server, &measurement)
                    .err()
                    .is_some()
                {
                    State::WifiReconnect
                } else {
                    led.set_pixel(RGB8::new(0, 50, 0))?;
                    std::thread::sleep(std::time::Duration::from_secs(u64::from(
                        app_config.read_interval,
                    )));
                    state
                };
                state = next_state;
            }
        }
    }
}

#[derive(Debug)]
enum State {
    Run,
    WifiReconnect,
}

fn should_include_radon(last: Option<PrimitiveDateTime>, current: PrimitiveDateTime) -> bool {
    warn!("last run {:?}, current run {:?}", last, current);
    if let Some(last) = last {
        last.hour() < current.hour()
    } else {
        true
    }
}

fn wait_for_sntp(sntp: &EspSntp) {
    info!("Waiting for sntp sync {:?}", sntp.get_sync_status());
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if sntp.get_sync_status() == SyncStatus::Completed {
            break;
        }
    }
}
