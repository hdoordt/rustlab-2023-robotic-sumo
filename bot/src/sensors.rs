use embassy_futures::yield_now;
use embassy_rp::{
    adc,
    gpio::{self, Level},
    peripherals::{PIN_0, PIN_1, PIN_27},
};

const LINE_COLOR_THRESHOLD: u16 = 141;

pub struct Sensors<'p> {
    adc: adc::Adc<'p, adc::Async>,
    distance_left: gpio::Input<'p, PIN_0>,
    distance_right: gpio::Input<'p, PIN_1>,
    line_digital: gpio::Input<'p, PIN_27>,
    line_analog: adc::Channel<'p>,
}

impl<'p> Sensors<'p> {
    pub fn new(
        adc: adc::Adc<'p, adc::Async>,
        distance_left: gpio::Input<'p, PIN_0>,
        distance_right: gpio::Input<'p, PIN_1>,
        line_digital: gpio::Input<'p, PIN_27>,
        line_analog: adc::Channel<'p>,
    ) -> Self {
        Self {
            adc,
            distance_left,
            distance_right,
            line_digital,
            line_analog,
        }
    }

    // pub async fn line_detected(&mut self) -> bool {
    //     matches!(self.line.get_level(), Level::Low)
    // }

    pub async fn line_dist(&mut self) -> u16 {
        self.adc.read(&mut self.line_analog).await.unwrap()
    }

    pub async fn line_detected(&mut self) -> bool {
        self.line_dist().await < LINE_COLOR_THRESHOLD
    }

    pub async fn detect_line(&mut self) {
        while !self.line_detected().await {}
    }

    pub fn line_detected_dig(&mut self) -> bool {
        matches!(self.line_digital.get_level(), Level::Low)
    }

    pub fn object_on_left(&mut self) -> bool {
        matches!(self.distance_left.get_level(), Level::Low)
    }

    pub fn object_on_right(&mut self) -> bool {
        matches!(self.distance_right.get_level(), Level::Low)
    }

    pub async fn detect_object_on_left(&mut self) {
        self.distance_left.wait_for_low().await
    }

    pub async fn detect_object_on_right(&mut self) {
        self.distance_right.wait_for_low().await
    }
}

pub struct DistanceSensor<'p, PIN: gpio::Pin> {
    pin: gpio::Input<'p, PIN>,
}

impl<'p, PIN: gpio::Pin> DistanceSensor<'p, PIN> {
    pub fn new(pin: gpio::Input<'p, PIN>) -> Self {
        Self { pin }
    }

    pub fn object_detected(&mut self) -> bool {
        matches!(self.pin.get_level(), Level::Low)
    }

    pub async fn detect_object(&mut self) {
        self.pin.wait_for_low().await
    }

    pub async fn detect_no_object(&mut self) {
        self.pin.wait_for_high().await
    }
}

pub struct LineSensor<'p> {
    adc: adc::Adc<'p, adc::Async>,
    line_digital: gpio::Input<'p, PIN_27>,
    line_analog: adc::Channel<'p>,
}

impl<'p> LineSensor<'p> {
    pub fn new(
        adc: adc::Adc<'p, adc::Async>,
        line_digital: gpio::Input<'p, PIN_27>,
        line_analog: adc::Channel<'p>,
    ) -> Self {
        Self {
            adc,
            line_digital,
            line_analog,
        }
    }

    pub async fn line_dist(&mut self) -> u16 {
        self.adc.read(&mut self.line_analog).await.unwrap()
    }

    pub async fn line_detected(&mut self) -> bool {
        self.line_dist().await < LINE_COLOR_THRESHOLD
    }

    pub async fn detect_line(&mut self) {
        while !self.line_detected().await {
            yield_now().await
        }
    }

    pub fn line_detected_dig(&mut self) -> bool {
        matches!(self.line_digital.get_level(), Level::Low)
    }
}
