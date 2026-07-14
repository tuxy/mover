use embedded_hal::{digital::OutputPin, pwm::SetDutyCycle};
use rp2040_hal::gpio::{DynPinId, FunctionSio, Pin, PullDown, SioOutput};

pub type ErasedOutputPin = Pin<DynPinId, FunctionSio<SioOutput>, PullDown>;

pub enum MotorDirection {
    Forward,
    Reverse,
}

pub struct OpenMotorController<P>
where
    P: SetDutyCycle,
{
    pwm_channel: P,
    dir_pins: [ErasedOutputPin; 2],
}

impl<P> OpenMotorController<P>
where
    P: SetDutyCycle,
{
    pub fn new(pwm_channel: P, dir_pins: [ErasedOutputPin; 2]) -> Self {
        Self {
            pwm_channel,
            dir_pins,
        }
    }

    pub fn set_percentage(&mut self, value: u16) {
        let _ = self.pwm_channel.set_duty_cycle(value);
    }

    pub fn set_direction(&mut self, direction: MotorDirection) {
        match direction {
            MotorDirection::Forward => {
                self.dir_pins[0].set_high();
                self.dir_pins[1].set_low();
            }
            MotorDirection::Reverse => {
                self.dir_pins[1].set_high();
                self.dir_pins[0].set_low();
            }
        }
    }
}
