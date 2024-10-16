use esp32_nimble::BLEAddress;
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use time::{format_description, PrimitiveDateTime};

use crate::utils::time::get_datetime;

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

#[derive(Debug, Serialize, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
pub struct MeasurementMetadata {
    serial_number: u32,
    address: BLEAddress,
    datetime: PrimitiveDateTime,
}

impl Serialize for MeasurementMetadata {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("WavePlusMeasurement", 3)?;

        let serial = self.serial_number.to_string();
        state.serialize_field("serial_number", &serial)?;

        let address = self.address.to_string();
        state.serialize_field("address", &address)?;

        let format = format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
            .expect("Failed to format time");
        let datetime = self
            .datetime
            .format(&format)
            .expect("Failed to format time");
        state.serialize_field("datetime", &datetime)?;

        state.end()
    }
}

#[derive(Debug, Serialize, Clone, Copy)]
pub struct WavePlusMeasurement {
    metadata: MeasurementMetadata,
    data: WavePlusMeasurementData,
}

impl WavePlusMeasurement {
    pub fn new(
        serial_number: u32,
        address: BLEAddress,
        data: &WavePlusRawMeasurementData,
        include_radon: bool,
    ) -> Self {
        let mut data = WavePlusMeasurementData::from(data);
        if !include_radon {
            log::warn!("Not returning radon measurement");
            data.radon_long = None;
            data.radon_short = None;
        }
        let datetime = get_datetime().expect("Unable to get current date and time");
        let metadata = MeasurementMetadata {
            serial_number,
            address,
            datetime,
        };
        WavePlusMeasurement { metadata, data }
    }

    pub fn has_radon(&self) -> bool {
        self.data.radon_short.is_some()
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
