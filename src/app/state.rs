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
            Status::Initializing => RGB8::new(50, 50, 0),
            Status::Ready => RGB8::new(0, 50, 0),
            Status::Collecting => RGB8::new(0, 0, 50),
            Status::Sending => RGB8::new(0, 50, 50),
            Status::Error => RGB8::new(50, 0, 0),
            Status::Recovering => RGB8::new(50, 0, 50),
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
}

impl State {
    pub fn with_mode(&self, mode: ExecutionMode) -> Self {
        State {
            mode,
            status: Status::from(mode),
            measurement: None,
            force_radon_measurement: false,
            ..*self
        }
    }

    pub fn with_last_run(&self, last_run: PrimitiveDateTime) -> Self {
        State {
            last_run: Some(last_run),
            measurement: None,
            ..*self
        }
    }

    pub fn force_radon_measurement(&self, force_radon_measurement: bool) -> Self {
        State {
            force_radon_measurement,
            measurement: None,
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
            measurement: None,
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
        }
    }
}
