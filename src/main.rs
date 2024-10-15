use anyhow::{bail, Result};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::prelude::Peripherals,
    // sys::esp_restart,
    wifi::{AuthMethod, WifiEvent},
};
use log::*;

use esp_idf_svc::netif::IpEvent;

mod ble;
mod http;
mod measurement;
mod rgbled;
mod wifi;

use ble::read_waveplus;
use http::send_measurement;
use rgbled::{RGB8, WS2812RMT};
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

    info!("******* Wifi: Subscribing to events");
    let _wifi_event_sub = sysloop.subscribe::<WifiEvent, _>(move |event| match event {
        WifiEvent::StaConnected => {
            info!("******* Received STA Connected Event");
            // core::internal::wifi::COMMAND.signal(core::internal::wifi::WifiCommand::StaConnected);
        }
        WifiEvent::StaDisconnected => {
            info!("******* Received STA Disconnected event");
        }
        _ => info!("Received other Wifi event: {:?}", event),
    })?;

    let _ip_event_sub = sysloop.subscribe::<IpEvent, _>(move |event| match event {
        _ => info!("Received other IPEvent: {:?}", event),
    })?;

    let wifi = connect_wifi(
        peripherals.modem,
        sysloop.clone(),
        None,
        AuthMethod::WPA2Personal,
        app_config.wifi_ssid,
        app_config.wifi_psk,
    )?;

    let serial: u32 = app_config.waveplus_serial.parse()?;
    let mut state: State = State::Init;
    loop {
        info!("Current state: {:?}", state);
        match state {
            State::Init => {
                info!("Initializing wifi");
                wait_for_connected(&wifi)?;
                led.set_pixel(RGB8::new(0, 50, 0))?;

                state = State::Run;
            }
            State::WifiReconnect => {
                led.set_pixel(RGB8::new(50, 50, 0))?;
                std::thread::sleep(std::time::Duration::from_millis(250));

                // warn!("Disconnecting wifi");
                // wifi.disconnect()?;
                // wait_for_disconnected(&wifi)?;

                // warn!("Restarting wifi");
                // wifi.start()?;

                // warn!("Reconnecing wifi");
                // wifi.connect()?;
                // wait_for_connected(&wifi)?;

                led.set_pixel(RGB8::new(0, 50, 0))?;

                state = State::Run;
            }
            State::Run => {
                let next_state: State;

                led.set_pixel(RGB8::new(0, 0, 50))?;
                let measurement = read_waveplus(&serial)?;
                if send_measurement(app_config.server, &measurement).err().is_some() {
                    // Red!
                    led.set_pixel(RGB8::new(50, 0, 0))?;
                    next_state = State::WifiReconnect;
                } else {
                    std::thread::sleep(std::time::Duration::from_secs(1));

                    // Green!
                    led.set_pixel(RGB8::new(0, 50, 0))?;

                    // Wait...
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    next_state = state;
                }
                state = next_state;
            }
        }
    }
}

#[derive(Debug)]
enum State {
    Init,
    Run,
    WifiReconnect,
}

// fn run() -> Result<()> {

//     Ok(())
// }
