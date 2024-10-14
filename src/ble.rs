use anyhow::{anyhow, Result};
use esp32_nimble::{uuid128, BLEAddress, BLEClient, BLEDevice, BLEAdvertisedDevice};
use esp_idf_svc::hal::task::block_on;
use log::*;
use bincode::Options;

use crate::measurement::{WavePlusManufacturerInfo, WavePlusMeasurement, WavePlusRawMeasurement};

macro_rules! bincode_options {
    () => {
        bincode::DefaultOptions::new()
        .with_little_endian()
        .with_no_limit()
        .with_fixint_encoding()
    };
}

fn parse_value(value: &Vec<u8>) -> Result<WavePlusMeasurement> {
    if value.len() != 20 {
        return Err(anyhow!("Unexpected BLE packet {:?}", value));
    }
    log::info!("can read {:?}", value);

    // <BBBBHHHHHHHH
    // [1, 96, 4, 0, 52, 0, 52, 0, 84, 7, 166, 196, 154, 2, 52, 0, 0, 0, 68, 7]

    let raw: WavePlusRawMeasurement;
    raw = bincode_options!().deserialize(&value).unwrap();

    let measurement = WavePlusMeasurement::from(raw);;

    log::info!("measurement: {:?}", measurement);
    Ok(measurement)
}


async fn read_waveplus_once(serial_number: &u32) -> Result<WavePlusMeasurement> {
    let ble_device = BLEDevice::take();
    let ble_scan = ble_device.get_scan();
    let device = ble_scan
        .active_scan(true)
        .interval(100)
        .window(99)
        .find_device(10000, |device| -> bool {
            if let Some(mfg_data) = device.get_manufacture_data() {
                if mfg_data.len() != 8 {
                    return false;
                }

                let mfg: WavePlusManufacturerInfo;
                mfg = bincode_options!().deserialize(&mfg_data).unwrap();

                info!("Found potential device: {:?}", mfg);

                // Magic constant to identify that this is a WavePlus device
                if mfg.manufacturer != 0x0334 {
                    return false;
                }

                mfg.serial_number == *serial_number
            } else {
                false
            }
        })
        .await?;

    if let Some(waveplus) = device {
        let mut client = BLEClient::new();
        client.on_connect(|client| {
            client.update_conn_params(120, 120, 0, 60).unwrap();
        });
        client.connect(waveplus.addr()).await?;

        let mut iter = client.get_services().await?;

        let service_uuid = uuid128!("b42e1c08-ade7-11e4-89d3-123b93f75cba");
        let characteristic_uuid = uuid128!("b42e2a68-ade7-11e4-89d3-123b93f75cba");

        let service = client
            .get_service(service_uuid)
            .await?;

        let characteristic = service
            .get_characteristic(characteristic_uuid)
            .await?;

        if !characteristic.can_read() {
            ::log::error!("characteristic can't read: {}", characteristic);
            client.disconnect()?;
            return Err(anyhow!("Unable to read measurement"));
        }

        let raw_value = characteristic.read_value().await;

        client.disconnect()?;

        match raw_value {
            Ok(value) => parse_value(&value),
            Err(_) => Err(anyhow!("Failed to read measurement")),
        }
    } else {
        Err(anyhow!("Could not find device"))
    }
}


pub fn read_waveplus(serial_number: &u32) -> Result<WavePlusMeasurement> {
    block_on(async {
        let measurement: WavePlusMeasurement = read_waveplus_once(serial_number).await?;
        Ok(measurement)
    })
}
