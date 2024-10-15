use serde::{Deserialize, Serialize};
use time::format_description;

use crate::time::get_datetime;

#[derive(Debug, Deserialize)]
pub struct WavePlusManufacturerInfo {
    pub serial_number: u32,
    pub _unknown: u16,
}

#[derive(Debug, Deserialize)]
pub struct WavePlusRawMeasurementData {
    version: u8,
    humidity: u8,
    _unknown1: u16,
    radon_short: u16,
    radon_long: u16,
    temperature: u16,
    pressure: u16,
    co2: u16,
    voc: u16,
    _unknown2: u32,
}

#[derive(Debug, Serialize)]
pub struct WavePlusMeasurementData {
    version: u8,
    humidity: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    radon_short: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    radon_long: Option<f64>,
    temperature: f64,
    pressure: f64,
    co2: f64,
    voc: f64,
}

#[derive(Debug, Serialize)]
pub struct WavePlusMeasurement {
    serial_number: String,
    address: String,
    datetime: Option<String>,
    data: WavePlusMeasurementData,
}

impl WavePlusMeasurement {
    pub fn new(
        serial_number: &u32,
        address: &str,
        data: &WavePlusRawMeasurementData,
        include_radon: bool,
    ) -> WavePlusMeasurement {
        let datetime: Option<String> =
            match format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]") {
                Ok(format) => match get_datetime() {
                    Ok(dt) => match dt.format(&format) {
                        Ok(s) => Some(s),
                        Err(_) => None,
                    },
                    Err(_) => None,
                },
                Err(_) => None,
            };
        let mut data = WavePlusMeasurementData::from(data);
        if !include_radon {
            log::warn!("Not returning radon measurement");
            data.radon_long = None;
            data.radon_short = None;
        }
        WavePlusMeasurement {
            serial_number: serial_number.to_string(),
            address: address.to_string(),
            datetime,
            data,
        }
    }
}

fn parse_radon(value: u16) -> Option<f64> {
    if value > 16383 {
        return None;
    }
    Some(f64::from(value))
}

impl From<&WavePlusRawMeasurementData> for WavePlusMeasurementData {
    fn from(raw: &WavePlusRawMeasurementData) -> WavePlusMeasurementData {
        let radon_short = parse_radon(raw.radon_short);
        let radon_long = parse_radon(raw.radon_long);

        WavePlusMeasurementData {
            version: raw.version,
            humidity: f64::from(raw.humidity) / 2.0,
            radon_short,
            radon_long,
            temperature: f64::from(raw.temperature) / 100.0,
            pressure: f64::from(raw.pressure) / 50.0,
            co2: f64::from(raw.co2),
            voc: f64::from(raw.voc),
        }
    }
}
