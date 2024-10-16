use serde::ser::{SerializeStruct, Serializer};
use serde::Serialize;
use time::PrimitiveDateTime;

use crate::rgbled::RGB8;
use crate::waveplus::measurement::WavePlusMeasurement;
use esp32_nimble::BLEAdvertisedDevice;

#[derive(Debug, Clone, Copy)]
pub enum ExecutionMode {
    Initialize,
    Reinitialize,
    CollectMeasurement,
    SendMeasurement,
    Wait,
    WifiDisconnect,
    WifiReconnect,
}

#[derive(Debug, Clone, Copy)]
pub enum Status {
    Initializing,
    Ready,
    Collecting,
    Sending,
    Error,
    Recovering,
}

impl From<ExecutionMode> for Status {
    fn from(mode: ExecutionMode) -> Status {
        match mode {
            ExecutionMode::Initialize => Status::Initializing,
            ExecutionMode::Reinitialize => Status::Recovering,
            ExecutionMode::CollectMeasurement => Status::Collecting,
            ExecutionMode::SendMeasurement => Status::Sending,
            ExecutionMode::Wait => Status::Ready,
            ExecutionMode::WifiDisconnect => Status::Error,
            ExecutionMode::WifiReconnect => Status::Recovering,
        }
    }
}

impl From<Status> for RGB8 {
    fn from(status: Status) -> RGB8 {
        match status {
            Status::Initializing => RGB8::new(10, 10, 0),
            Status::Ready => RGB8::new(0, 10, 0),
            Status::Collecting => RGB8::new(0, 0, 10),
            Status::Sending => RGB8::new(0, 10, 10),
            Status::Error => RGB8::new(10, 0, 0),
            Status::Recovering => RGB8::new(10, 0, 10),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, Serialize)]
struct Errors {
    wifi_disconnects: u64,
    ble_disconnects: u64,
    http_errors: u64,
}

impl Errors {
    fn wifi_disconnected(&self) -> Self {
        Errors {
            wifi_disconnects: self.wifi_disconnects + 1,
            ..*self
        }
    }

    fn ble_disconnected(&self) -> Self {
        Errors {
            ble_disconnects: self.ble_disconnects + 1,
            ..*self
        }
    }

    fn http_error(&self) -> Self {
        Errors {
            http_errors: self.http_errors + 1,
            ..*self
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub mode: ExecutionMode,
    pub status: Status,
    pub last_run: Option<PrimitiveDateTime>,
    pub measurement: Option<WavePlusMeasurement>,
    pub force_radon_measurement: bool,
    pub waveplus: Option<BLEAdvertisedDevice>,
    errors: Errors,
}

impl Serialize for State {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("State", 4)?;

        if let Some(measurement) = self.measurement {
            state.serialize_field("metadata", &measurement.metadata)?;
            state.serialize_field("data", &measurement.data)?;
        }

        state.serialize_field("measurement", &self.measurement)?;
        state.serialize_field("errors", &self.errors)?;

        state.end()
    }
}

impl State {
    pub fn wifi_disconnected(&self) -> Self {
        State {
            errors: self.errors.wifi_disconnected(),
            ..*self
        }
    }

    pub fn ble_disconnected(&self) -> Self {
        State {
            errors: self.errors.ble_disconnected(),
            ..*self
        }
    }

    pub fn http_error(&self) -> Self {
        State {
            errors: self.errors.http_error(),
            ..*self
        }
    }

    pub fn with_mode(&self, mode: ExecutionMode) -> Self {
        State {
            mode,
            status: Status::from(mode),
            measurement: None,
            force_radon_measurement: false,
            ..*self
        }
    }

    pub fn measurement_has_radon(&self) -> bool {
        if let Some(measurement) = self.measurement {
            measurement.has_radon()
        } else {
            false
        }
    }

    pub fn with_last_run(&self, last_run: PrimitiveDateTime) -> Self {
        State {
            last_run: Some(last_run),
            ..*self
        }
    }

    pub fn force_radon_measurement(&self, force_radon_measurement: bool) -> Self {
        State {
            force_radon_measurement,
            ..*self
        }
    }

    pub fn with_measurement(&self, measurement: WavePlusMeasurement) -> Self {
        State {
            measurement: Some(measurement),
            ..*self
        }
    }

    pub fn with_waveplus(&self, waveplus: BLEAdvertisedDevice) -> Self {
        State {
            waveplus: Some(waveplus),
            ..*self
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            mode: ExecutionMode::Initialize,
            status: Status::Ready,
            last_run: None,
            measurement: None,
            force_radon_measurement: true,
            waveplus: None,
            errors: Errors::default(),
        }
    }
}
