use anyhow::Result;
use esp_idf_svc::hal::modem::WifiModemPeripheral;
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi},
};
use log::*;

// use crate::wifi_fix::WifiConnectFix;

pub fn connect_wifi<'d>(
    modem: impl Peripheral<P = impl WifiModemPeripheral + 'd> + 'd,
    sysloop: EspSystemEventLoop,
    partition: Option<EspDefaultNvsPartition>,
    auth_method: AuthMethod,
    ssid: &str,
    psk: &str,
) -> Result<EspWifi<'d>> {
    let mut wifi = EspWifi::new(modem, sysloop.clone(), partition)?;

    if psk.is_empty() {
        wifi.set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: ssid.try_into().expect("Could not parse SSID"),
            auth_method: AuthMethod::None,
            ..Default::default()
        }))?;
    } else {
        wifi.set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: ssid
                .try_into()
                .expect("Could not parse SSID into Wifi config"),
            password: psk
                .try_into()
                .expect("Could not parse PSK into Wifi config"),
            auth_method,
            ..Default::default()
        }))?;
    }

    wifi.start()?;
    wifi.connect()?;

    Ok(wifi)
}

pub fn wait_for_connected(wifi: &EspWifi) -> Result<()> {
    loop {
        std::thread::sleep(std::time::Duration::from_millis(250));
        let connected = wifi.is_up()?;
        if connected {
            break;
        }
    }

    info!("Connected to wifi");

    Ok(())
}
