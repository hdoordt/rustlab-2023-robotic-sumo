use embassy_rp::{
    peripherals::{PWM_CH1, PWM_CH3},
    pwm,
};
use fixed::traits::ToFixed;

const MAX_DUTY: u16 = 3500;
const PWN_DIV_INT: u8 = 250;
const PWM_TOP: u16 = 10000;

pub struct Motors<'p> {
    left: pwm::Pwm<'p, PWM_CH3>,
    right: pwm::Pwm<'p, PWM_CH1>,
}

impl<'p> Motors<'p> {
    pub fn new(left: pwm::Pwm<'p, PWM_CH3>, right: pwm::Pwm<'p, PWM_CH1>) -> Self {
        Self { left, right }
    }

    pub fn set_left_speed(&mut self, speed: i16) {
        self.left.set_config(&Self::conf_for_speed(speed));
    }

    pub fn set_right_speed(&mut self, speed: i16) {
        self.right.set_config(&Self::conf_for_speed(speed));
    }

    pub fn turn_left(&mut self) {
        self.set_left_speed(-3500);
        self.set_right_speed(3500);
    }

    pub fn turn_right(&mut self) {
        self.set_left_speed(3500);
        self.set_right_speed(-3500);
    }

    #[allow(non_snake_case)]
    pub fn CHARGEEEEE(&mut self) {
        self.set_left_speed(3500);
        self.set_right_speed(3500);
    }

    pub fn reverse(&mut self) {
        self.set_left_speed(-3500);
        self.set_right_speed(-3500);
    }

    pub fn conf_for_speed(speed: i16) -> pwm::Config {
        let speed = speed.min(MAX_DUTY as i16).max(-(MAX_DUTY as i16));

        let (duty_a, duty_b) = if speed < 0 {
            (speed.abs() as u16, 0)
        } else {
            (0, speed.abs() as u16)
        };

        Self::pwm_config(duty_a, duty_b)
    }

    fn pwm_config(duty_a: u16, duty_b: u16) -> pwm::Config {
        let mut c = pwm::Config::default();
        c.invert_a = false;
        c.invert_b = false;
        c.phase_correct = false;
        c.enable = true;
        c.divider = PWN_DIV_INT.to_fixed();
        c.compare_a = duty_a;
        c.compare_b = duty_b;
        c.top = PWM_TOP;
        c
    }
}
