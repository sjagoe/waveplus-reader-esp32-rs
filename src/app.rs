use anyhow::Result;
use esp_idf_svc::wifi::EspWifi;
use log::*;
use time::PrimitiveDateTime;

mod http;
mod state;

use crate::app::state::*;
use crate::rgbled::{RGB8, WS2812RMT};
use crate::utils::time::get_datetime;
use crate::waveplus::{get_waveplus, read_waveplus};
use crate::wifi::wait_for_connected;

pub use crate::app::state::Status;

fn should_include_radon(last: Option<PrimitiveDateTime>, current: PrimitiveDateTime) -> bool {
    warn!("last run {:?}, current run {:?}", last, current);
    if let Some(last) = last {
        last.hour() < current.hour()
    } else {
        true
    }
}

pub fn run(
    wifi: &mut EspWifi,
    led: &mut WS2812RMT,
    serial: u32,
    server: &str,
    read_interval: u16,
) -> Result<()> {
    let mut state: State = State::default();
    loop {
        led.set_pixel(RGB8::from(state.status))?;
        info!("Current state: {:?}", state);
        state = match state.mode {
            ExecutionMode::Initialize | ExecutionMode::Reinitialize => {
                let waveplus = get_waveplus(&serial).expect("Unable to get waveplus bt device");
                let newstate = state
                    .with_mode(ExecutionMode::CollectMeasurement)
                    .with_waveplus(waveplus);
                match state.mode {
                    ExecutionMode::Reinitialize => newstate.ble_disconnected(),
                    _ => newstate,
                }
            }
            ExecutionMode::WifiDisconnect => {
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
                state.with_mode(ExecutionMode::WifiReconnect).wifi_disconnected()
            }
            ExecutionMode::WifiReconnect => {
                warn!(
                    "Wifi connected: {:?}, up: {:?}",
                    wifi.is_connected()?,
                    wifi.is_up()?
                );

                wait_for_connected(wifi)?;

                state.with_mode(ExecutionMode::CollectMeasurement)
            }
            ExecutionMode::CollectMeasurement => {
                let current = get_datetime()?;
                let include_radon =
                    should_include_radon(state.last_run, current) || state.force_radon_measurement;

                if let Some(waveplus) = state.waveplus {
                    warn!("Include radon measurement? {:?}", include_radon);
                    match read_waveplus(serial, &waveplus, include_radon) {
                        Ok(measurement) => state
                            .with_mode(ExecutionMode::SendMeasurement)
                            .with_measurement(measurement),
                        Err(err) => {
                            error!("Failed to retrieve data from {:?}: {:?}", waveplus, err);
                            state.with_mode(ExecutionMode::Reinitialize)
                        }
                    }
                } else {
                    state.with_mode(ExecutionMode::Reinitialize)
                }
            }
            ExecutionMode::SendMeasurement => {
                let current = get_datetime()?;
                let newstate = if http::send(&state, server).err().is_some() {
                    state
                        .with_mode(ExecutionMode::WifiDisconnect)
                        .force_radon_measurement(state.measurement_has_radon())
                        .http_error()
                } else {
                    state.with_mode(ExecutionMode::Wait)
                };
                newstate.with_last_run(current)
            }
            ExecutionMode::Wait => {
                std::thread::sleep(std::time::Duration::from_secs(u64::from(read_interval)));
                state.with_mode(ExecutionMode::CollectMeasurement)
            }
        };
    }
}
