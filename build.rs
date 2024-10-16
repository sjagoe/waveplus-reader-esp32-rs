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

fn main() {
    let _app_config = CONFIG;

    embuild::espidf::sysenv::output();
}
