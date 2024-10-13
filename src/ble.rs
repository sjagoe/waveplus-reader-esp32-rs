use anyhow::{bail, Result};
use esp32_nimble::{uuid128, BLEAddress, BLEClient, BLEDevice, BLEAdvertisedDevice};
use esp_idf_svc::hal::{
  prelude::Peripherals,
  task::block_on,
  timer::{TimerConfig, TimerDriver},
};
use log::*;
use bincode::Options;

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

#[derive(Debug)]
pub struct WavePlusMeasurement {
    version: u8,
    humidity: f64,
    radon_short: Option<f64>,
    radon_long: Option<f64>,
    temperature: f64,
    pressure: f64,
    co2: f64,
    voc: f64,
}

fn parse_radon(value: u16) -> Option<f64> {
    if value > 16383 {
        return None;
    }
    Some(f64::from(value))
}

fn parse_value(value: &Vec<u8>) -> Result<()> {
    if value.len() != 20 {
        bail!("Unexpected BLE packet {:?}", value);
    }
    log::info!("can read {:?}", value);
    let bincode_options = bincode::DefaultOptions::new()
        .with_little_endian()
        .with_no_limit()
        .with_fixint_encoding() ;

    // <BBBBHHHHHHHH
    // [1, 96, 4, 0, 52, 0, 52, 0, 84, 7, 166, 196, 154, 2, 52, 0, 0, 0, 68, 7]

    let radon_short = parse_radon(bincode_options.deserialize::<u16>(&value[4 .. 6]).unwrap());
    let radon_long = parse_radon(bincode_options.deserialize::<u16>(&value[6 .. 8]).unwrap());

    let measurement = WavePlusMeasurement {
        version: bincode_options.deserialize::<u8>(&value[0 .. 1]).unwrap(),
        humidity: f64::from(bincode_options.deserialize::<u8>(&value[1 .. 2]).unwrap()) / 2.0,
        // ignore [2 .. 3], [3 .. 4]
        radon_short,
        radon_long,
        temperature: f64::from(bincode_options.deserialize::<u16>(&value[8 .. 10]).unwrap()) / 100.0,
        pressure: f64::from(bincode_options.deserialize::<u16>(&value[10 .. 12]).unwrap()) / 50.0,
        co2: f64::from(bincode_options.deserialize::<u16>(&value[12 .. 14]).unwrap()),
        voc: f64::from(bincode_options.deserialize::<u16>(&value[14 .. 16]).unwrap()),
    };

    log::info!("version: {:?}", measurement);
    Ok(())
}


pub fn get_waveplus(serial_number: &u64) -> Result<()> {
    let bincode_options = bincode::DefaultOptions::new()
        .with_little_endian()
        .with_no_limit()
        .with_fixint_encoding() ;

    // let peripherals = Peripherals::take()?;
    // let mut timer = TimerDriver::new(peripherals.timer00, &TimerConfig::new())?;
    block_on(async {
        let ble_device = BLEDevice::take();
        let ble_scan = ble_device.get_scan();
        let device = ble_scan
            .active_scan(true)
            .interval(100)
            .window(99)
            .find_device(10000, |device| -> bool {
                if let Some(mfg_data) = device.get_manufacture_data() {
                    if mfg_data.len() < 6 {
                        return false;
                    }

                    // let mfg: u16 = u16::from(mfg_data[1]) << 8_u8 | u16::from(mfg_data[0]);

                    // let mfg1: u16 = unpack_u16(&mfg_data[0 .. 2]).unwrap();

                    let mfg: u16 = bincode_options.deserialize(&mfg_data[0 .. 2]).unwrap();

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
