use anyhow::{bail, Result};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::prelude::Peripherals,
};

mod ble;
mod rgbled;
mod wifi;
mod http;

use ble::{read_waveplus, get_waveplus};
use rgbled::{RGB8, WS2812RMT};
use wifi::wifi;
use http::get;

/// This configuration is picked up at compile time by `build.rs` from the
/// file `cfg.toml`.
#[toml_cfg::toml_config]
pub struct Config {
    #[default("Wokwi-GUEST")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
    #[default("")]
    waveplus_serial: &'static str,
}

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

    log::info!("Hello, world!");

    // Start the LED off yellow
    let mut led = WS2812RMT::new(peripherals.pins.gpio8, peripherals.rmt.channel0)?;
    led.set_pixel(RGB8::new(50, 50, 0))?;

    // The constant `CONFIG` is auto-generated by `toml_config`.
    let app_config = CONFIG;

    log::info!("SSID: {:?}", app_config.wifi_ssid);

    // let _wifi = match wifi(
    //     app_config.wifi_ssid,
    //     app_config.wifi_psk,
    //     peripherals.modem,
    //     sysloop,
    // ) {
    //     Ok(inner) => inner,
    //     Err(err) => {
    //         // Red!
    //         led.set_pixel(RGB8::new(50, 0, 0))?;
    //         bail!("Could not connect to Wi-Fi network: {:?}", err)
    //     }
    // };

    // get("https://espressif.com/")?;

    let serial: u64 = app_config.waveplus_serial.parse()?;
    get_waveplus(&serial)?;
    // log::info!("got waveplus {:?}", waveplus);
    // read_waveplus(waveplus)?;

    loop {
        // Blue!
        led.set_pixel(RGB8::new(0, 0, 50))?;
        // Wait...
        std::thread::sleep(std::time::Duration::from_secs(1));
        log::info!("Hello, world!");

        // Green!
        led.set_pixel(RGB8::new(0, 50, 0))?;
        // Wait...
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
