use anyhow::{bail, Result};
use core::str;
use embedded_svc::http::{client::Client, Method};
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};

use crate::waveplus::measurement::WavePlusMeasurement;

pub fn send_measurement(url: impl AsRef<str>, measurement: &WavePlusMeasurement) -> Result<()> {
    // 1. Create a new EspHttpClient. (Check documentation)
    // ANCHOR: connection
    let connection = EspHttpConnection::new(&Configuration {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        timeout: Some(std::time::Duration::from_secs(30)),
        ..Default::default()
    })?;
    // ANCHOR_END: connection
    let mut client = Client::wrap(connection);

    // 2. Open a GET request to `url`
    let headers = [("content-type", "application/json")];
    let mut request = client.request(Method::Post, url.as_ref(), &headers)?;
    let json = serde_json::to_string(measurement)?;
    request.write(json.as_bytes())?;

    // 3. Submit write request and check the status code of the response.
    // Successful http status codes are in the 200..=299 range.
    let response = request.submit()?;
    let status = response.status();

    println!("Response code: {}\n", status);

    match status {
        200..=299 => Ok(()),
        _ => bail!("Unexpected response code: {}", status),
    }
}
