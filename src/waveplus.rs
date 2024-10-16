use anyhow::{anyhow, Result};
use bincode::Options;
use esp32_nimble::{uuid128, BLEAdvertisedDevice, BLEClient, BLEDevice, BLEScan};
use esp_idf_svc::hal::task::block_on;
use log::*;

pub mod measurement;

use measurement::{WavePlusManufacturerInfo, WavePlusMeasurement, WavePlusRawMeasurementData};

macro_rules! bincode_options {
    () => {
        bincode::DefaultOptions::new()
            .with_little_endian()
            .with_no_limit()
            .with_fixint_encoding()
    };
}

fn parse_value(value: &Vec<u8>) -> Result<WavePlusRawMeasurementData> {
    if value.len() != 20 {
        return Err(anyhow!("Unexpected BLE packet {:?}", value));
    }
    let raw: WavePlusRawMeasurementData = bincode_options!().deserialize(value).unwrap();
    Ok(raw)
}

pub fn get_waveplus(serial_number: &u32) -> Result<BLEAdvertisedDevice> {
    info!("Scanning for Wave Plus devices");
    block_on(async {
        let ble_device = BLEDevice::take();
        let mut ble_scan = BLEScan::new();
        let device = ble_scan
            .active_scan(true)
            .interval(100)
            .window(99)
            .start(ble_device, 10000, |device, data| {
                if let Some(manufacture_data) = data.manufacture_data() {
                    if manufacture_data.company_identifier != 0x0334 {
                        return None::<BLEAdvertisedDevice>;
                    }
                    let mfg: WavePlusManufacturerInfo = bincode_options!()
                        .deserialize(manufacture_data.payload)
                        .unwrap();
                    if mfg.serial_number == *serial_number {
                        return Some(*device);
                    }
                }
                None::<BLEAdvertisedDevice>
            })
            .await?;

        if let Some(device) = device {
            Ok(device)
        } else {
            Err(anyhow!(
                "Could not find Wave Plus with serial {:?}",
                serial_number
            ))
        }
    })
}

pub fn read_waveplus(
    serial_number: u32,
    waveplus: &BLEAdvertisedDevice,
    include_radon: bool,
) -> Result<WavePlusMeasurement> {
    info!(
        "Scraping measurement from {:?}: {:?}",
        serial_number, waveplus
    );
    block_on(async {
        let mut client = BLEClient::new();
        client.on_connect(|client| {
            client.update_conn_params(120, 120, 0, 60).unwrap();
        });
        client.connect(&waveplus.addr()).await?;

        let service_uuid = uuid128!("b42e1c08-ade7-11e4-89d3-123b93f75cba");
        let characteristic_uuid = uuid128!("b42e2a68-ade7-11e4-89d3-123b93f75cba");

        let service = client.get_service(service_uuid).await?;

        let characteristic = service.get_characteristic(characteristic_uuid).await?;

        if !characteristic.can_read() {
            error!("characteristic can't read: {}", characteristic);
            client.disconnect()?;
            return Err(anyhow!("Unable to read measurement"));
        }

        let raw_value = characteristic.read_value().await;

        client.disconnect()?;

        match raw_value {
            Ok(value) => {
                let raw = parse_value(&value)?;
                let measurement =
                    WavePlusMeasurement::new(serial_number, waveplus.addr(), &raw, include_radon);
                Ok(measurement)
            }
            Err(_) => Err(anyhow!("Failed to read measurement")),
        }
    })
}
