#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

mod motors;
mod sensors;
mod tcp_client;

use cyw43_pio::PioSpi;
use embassy_executor::Spawner;
use embassy_futures::select::select4;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler as InterruptHandlerPio, Pio};
use embassy_rp::{
    adc::{Adc, Channel, Config as ConfigAdc, InterruptHandler as InterruptHandlerAdc},
    bind_interrupts,
    gpio::{Input, Pull},
    peripherals::USB,
    pwm::Pwm,
    usb::{Driver, InterruptHandler as InterruptHandlerUsb},
};
use embassy_time::{Duration, Timer};
use rp2040_panic_usb_boot as _;
use sensors::Sensors;

use crate::tcp_client::Connection;
use crate::{
    motors::Motors,
    sensors::{DistanceSensor, LineSensor},
};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandlerUsb<USB>;
    ADC_IRQ_FIFO => InterruptHandlerAdc;
    PIO0_IRQ_0 => InterruptHandlerPio<PIO0>;
});

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Debug, driver);
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Init USB logger
    let driver = Driver::new(p.USB, Irqs);
    spawner.spawn(logger_task(driver)).unwrap();

    log::info!("Hello, world!");

    // init TCP Client
    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    // let mut conn = Connection::init(&spawner, pwr, spi).await;
    // conn.connect().await;

    // Init ADC
    let adc = Adc::new(p.ADC, Irqs, ConfigAdc::default());

    // Init sensor pins
    let left_dist = Input::new(p.PIN_0, Pull::None);
    let right_dist = Input::new(p.PIN_1, Pull::None);
    let line_digital = Input::new(p.PIN_27, Pull::None);
    let line_analog = Channel::new_pin(p.PIN_26, Pull::None);

    let mut line_sensor = LineSensor::new(adc, line_digital, line_analog);
    let mut left_dist = DistanceSensor::new(left_dist);
    let mut right_dist = DistanceSensor::new(right_dist);

    // Init motor pins
    let left_motor = Pwm::new_output_ab(p.PWM_CH3, p.PIN_6, p.PIN_7, Motors::conf_for_speed(0));
    let right_motor = Pwm::new_output_ab(p.PWM_CH1, p.PIN_2, p.PIN_3, Motors::conf_for_speed(0));
    let mut motors = Motors::new(left_motor, right_motor);

    loop {
        log::info!("Loopsy");
        // Timer::after(Duration::from_millis(1)).await;
        match select4(
            line_sensor.detect_line(),
            left_dist.detect_object(),
            right_dist.detect_object(),
            Timer::after(Duration::from_millis(1)),
        )
        .await
        {
            embassy_futures::select::Either4::First(_) => {
                motors.reverse();
                log::info!("LINE");
                // line detected
            }
            embassy_futures::select::Either4::Second(_) => {
                // something on the left detected

                if right_dist.object_detected() {
                    log::info!("CHARGE!!!");
                    motors.CHARGEEEEE();
                } else {
                    log::info!("LEFT");
                    motors.turn_left();
                }
            }
            embassy_futures::select::Either4::Third(_) => {
                // something on the right detected
                motors.turn_right();
                log::info!("RIGHT");
            }
            embassy_futures::select::Either4::Fourth(_) => {
                // nothing detected
                motors.turn_left();
                log::info!("NOTHING");
            }
        };
    }
}
