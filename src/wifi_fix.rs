use esp_idf_svc::wifi::{BlockingWifi, EspWifi};

pub trait WifiConnectFix {
    fn connect_with_retry(&mut self) -> anyhow::Result<()>;
}

fn sleep_ms(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

impl WifiConnectFix for BlockingWifi<&mut EspWifi<'_>> {
    fn connect_with_retry(&mut self) -> anyhow::Result<()> {
        let mut retry_delay_ms = 1_000;
        loop {
            log::info!("Connecting wifi...");
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

        log::info!("Waiting for DHCP lease...");

        self.wait_netif_up()?;
        Ok(())
    }
}
