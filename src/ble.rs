use anyhow::{bail, Result};
use esp32_nimble::{uuid128, BLEAddress, BLEClient, BLEDevice, BLEAdvertisedDevice};
use esp_idf_svc::hal::task::block_on;
use log::*;
use bincode::Options;

use crate::measurement::{WavePlusMeasurement, WavePlusRawMeasurement};

macro_rules! bincode_options {
    () => {
        bincode::DefaultOptions::new()
        .with_little_endian()
        .with_no_limit()
        .with_fixint_encoding()
    };
}

pub fn scan_ble() -> anyhow::Result<()> {
    block_on(async {
        let ble_device = BLEDevice::take();
        let ble_scan = ble_device.get_scan();
        ble_scan
            .active_scan(true)
            .interval(100)
            .window(99)
            .on_result(|_scan, param| {
                info!("Advertised Device: {:?}", param);
            });
        ble_scan.start(5000).await?;
        info!("Scan end");

        Ok(())
    })
}

fn parse_value(value: &Vec<u8>) -> Result<()> {
    if value.len() != 20 {
        bail!("Unexpected BLE packet {:?}", value);
    }
    log::info!("can read {:?}", value);

    // <BBBBHHHHHHHH
    // [1, 96, 4, 0, 52, 0, 52, 0, 84, 7, 166, 196, 154, 2, 52, 0, 0, 0, 68, 7]

    let raw: WavePlusRawMeasurement;
    raw = bincode_options!().deserialize(&value).unwrap();

    let measurement = WavePlusMeasurement::from(raw);;

    log::info!("measurement: {:?}", measurement);
    Ok(())
}


pub fn get_waveplus(serial_number: &u64) -> Result<()> {
    block_on(async {
        let ble_device = BLEDevice::take();
        let ble_scan = ble_device.get_scan();
        let device = ble_scan
            .active_scan(true)
            .interval(100)
            .window(99)
            .find_device(10000, |device| -> bool {
                if let Some(mfg_data) = device.get_manufacture_data() {
                    if mfg_data.len() != 6 {
                        return false;
                    }

                    let mfg: u16 = bincode_options!().deserialize(&mfg_data[0 .. 2]).unwrap();

                    // Magic constant to identify that this is a WavePlus device
                    if mfg != 0x0334 {
                        return false;
                    }

                    let serial: u64 = u64::from(mfg_data[2]) | u64::from(mfg_data[3]) << 8 | u64::from(mfg_data[4]) << 16 | u64::from(mfg_data[5]) << 24;
                    info!("Device? {:?} {:?} == {:?}", device, serial, serial_number);
                    serial == *serial_number
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
                return anyhow::Ok(());
            }

            let packet = match characteristic.read_value().await {
                Ok(value) => parse_value(&value),
                Err(_) => bail!("Failed to read value"),
            };

            client.disconnect()?;
            Ok(())
        } else {
            bail!("Could not find device")
        }
    })
}


pub fn read_waveplus(waveplus: BLEAdvertisedDevice) -> Result<()> {
    block_on(async {
        let ble_device = BLEDevice::take();
        let mut client = BLEClient::new();
        client.on_connect(|client| {
            client.update_conn_params(120, 120, 0, 60).unwrap();
        });
        client.connect(waveplus.addr()).await.unwrap();

        // let mut iter = client.get_services().await.unwrap();

        // iter.for_each(|x| info!("service {:?}", x));

        // let service = client
        //     .get_service(uuid128!("b42e2a68-ade7-11e4-89d3-123b93f75cba"))
        //     .await?;

        // info!("{:?}", service);
        client.disconnect()?;

        anyhow::Ok(())
    });
    return anyhow::Ok(());
}
