use anyhow::{bail, Result};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::peripheral,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use log::info;


pub trait WifiConnectFix {
    fn connect_with_retry(&mut self) -> anyhow::Result<()>;
}

pub fn sleep_ms(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

impl WifiConnectFix for BlockingWifi<&mut EspWifi<'_>> {
    fn connect_with_retry(&mut self) -> anyhow::Result<()> {
        let mut retry_delay_ms = 1_000;
        loop {
            info!("Connecting wifi...");
            match self.connect() {
                Ok(()) => break,
                Err(e) => {
                    log::warn!(
                        "Wifi connect failed, reason {}, retrying in {}s",
                        e,
                        retry_delay_ms / 1000
                    );
                    sleep_ms(retry_delay_ms);

                    // increase the delay exponentially, but cap it at 10s
                    retry_delay_ms = std::cmp::min(retry_delay_ms * 2, 10_000);

                    self.stop()?;
                    self.start()?;
                }
            }
        }

        info!("Waiting for DHCP lease...");

        self.wait_netif_up()?;
        Ok(())
    }
}

pub fn wifi(
    ssid: &str,
    pass: &str,
    modem: impl peripheral::Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
) -> Result<Box<EspWifi<'static>>> {
    let mut auth_method = AuthMethod::WPA2Personal;
    if ssid.is_empty() {
        bail!("Missing WiFi name")
    }
    if pass.is_empty() {
        auth_method = AuthMethod::None;
        info!("Wifi password is empty");
    }
    let mut esp_wifi = EspWifi::new(modem, sysloop.clone(), None)?;

    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;

    info!("Starting wifi...");

    wifi.start()?;

    info!("Scanning...");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| a.ssid == ssid);

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            ssid, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            ssid
        );
        None
    };

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid
            .try_into()
            .expect("Could not parse the given SSID into WiFi config"),
        password: pass
            .try_into()
            .expect("Could not parse the given password into WiFi config"),
        channel,
        auth_method,
        ..Default::default()
    }))?;

    wifi.start()?;

    wifi.connect_with_retry()?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    info!("Wifi DHCP info: {:?}", ip_info);

    Ok(Box::new(esp_wifi))
}
