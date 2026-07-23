#![no_std]
#![no_main]

mod motor;
mod system;
mod usb;

use core::cell::RefCell;
use critical_section::Mutex;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use hal::pac::interrupt;
use panic_halt as _;
use rp2040_hal::{self as hal, gpio};

use crate::motor::{ErasedOutputPin, MotorDirection, OpenMotorController};
use crate::system::System;
use crate::usb::UsbSerial;

const PWM_TOP: u16 = 65535;
const PPR: f32 = 468.0;

#[link_section = ".boot2"]
#[used]
static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

type EncoderPinA = gpio::Pin<gpio::bank0::Gpio8, gpio::FunctionSioInput, gpio::PullUp>;
type EncoderPinB = gpio::Pin<gpio::bank0::Gpio9, gpio::FunctionSioInput, gpio::PullUp>;
type EncoderPins = (EncoderPinA, EncoderPinB);

static ENCODER_PINS: Mutex<RefCell<Option<EncoderPins>>> = Mutex::new(RefCell::new(None));
static ENC_A_COUNTER: Mutex<RefCell<u32>> = Mutex::new(RefCell::new(0));
static ENC_B_COUNTER: Mutex<RefCell<u32>> = Mutex::new(RefCell::new(0));

macro_rules! setup_motor {
    ($name:ident, $pwm:expr, $chan:ident, $ena_pin:expr, [$in1:expr, $in2:expr]) => {
        let pwm = &mut $pwm;
        pwm.default_config();
        pwm.set_top(PWM_TOP);
        let en = &mut pwm.$chan;
        en.output_to($ena_pin);
        let pins: [ErasedOutputPin; 2] = [$in1.into_dyn_pin(), $in2.into_dyn_pin()];
        let mut $name = OpenMotorController::new(en, pins);
    };
}

#[hal::entry]
fn main() -> ! {
    let mut system = System::init();

    // Steal PAC for peripherals consumed after System::init
    let mut pac = unsafe { hal::pac::Peripherals::steal() };

    // Encoder inputs
    let enc_a = system.pins.gpio8.into_pull_up_input();
    let enc_b = system.pins.gpio9.into_pull_up_input();

    enc_a.set_interrupt_enabled(gpio::Interrupt::EdgeHigh, true);
    enc_a.set_interrupt_enabled(gpio::Interrupt::EdgeLow, true);
    enc_b.set_interrupt_enabled(gpio::Interrupt::EdgeHigh, true);
    enc_b.set_interrupt_enabled(gpio::Interrupt::EdgeLow, true);

    critical_section::with(|cs| {
        ENCODER_PINS.borrow(cs).replace(Some((enc_a, enc_b)));
    });

    unsafe {
        hal::pac::NVIC::unmask(hal::pac::Interrupt::IO_IRQ_BANK0);
    }

    // USB serial
    let mut terminal = UsbSerial::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        system.clocks.usb_clock,
        &mut pac.RESETS,
    );

    // Motors
    let mut pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);

    setup_motor!(
        motora_controller,
        pwm_slices.pwm5,
        channel_a,
        system.pins.gpio10,
        [
            system.pins.gpio11.into_push_pull_output(),
            system.pins.gpio12.into_push_pull_output()
        ]
    );

    setup_motor!(
        motorb_controller,
        pwm_slices.pwm6,
        channel_b,
        system.pins.gpio13,
        [
            system.pins.gpio14.into_push_pull_output(),
            system.pins.gpio15.into_push_pull_output()
        ]
    );

    // Main loop
    let mut led_pin = system.pins.gpio25.into_push_pull_output();
    let mut counter: u32 = 0;

    loop {
        terminal.poll();

        motora_controller.set_percentage(PWM_TOP / 2);
        motora_controller.set_direction(MotorDirection::Forward);
        motorb_controller.set_percentage(PWM_TOP / 2);
        motorb_controller.set_direction(MotorDirection::Reverse);

        counter += 1;
        if terminal.is_configured() && counter % 10 == 0 {
            let mut buffer_a = itoa::Buffer::new();
            let mut buffer_b = itoa::Buffer::new();

            let (enc_a, enc_b) = critical_section::with(|cs| {
                (
                    *ENC_A_COUNTER.borrow(cs).borrow(),
                    *ENC_B_COUNTER.borrow(cs).borrow(),
                )
            });

            let start = "Encoder values: {";
            let s_a = buffer_a.format(enc_a);
            let s_b = buffer_b.format(enc_b);

            match terminal.write(start.as_bytes()) {
                Ok(_) => {
                    let _ = terminal.write(s_a.as_bytes());
                    let _ = terminal.write(b", ");
                    let _ = terminal.write(s_b.as_bytes());
                    let _ = terminal.write(b"}\n");
                    let _ = led_pin.set_high();
                }
                _ => {
                    let _ = led_pin.set_low();
                }
            }
        }

        system.timer.delay_ms(10);
    }
}

#[allow(static_mut_refs)]
#[interrupt]
fn IO_IRQ_BANK0() {
    static mut PINS: Option<EncoderPins> = None;

    if PINS.is_none() {
        critical_section::with(|cs| {
            *PINS = ENCODER_PINS.borrow(cs).take();
        });
    }

    if let Some((enc_a, enc_b)) = PINS {
        if enc_a.interrupt_status(gpio::Interrupt::EdgeHigh) {
            critical_section::with(|cs| *ENC_A_COUNTER.borrow(cs).borrow_mut() += 1);
            enc_a.clear_interrupt(gpio::Interrupt::EdgeHigh);
        }
        if enc_a.interrupt_status(gpio::Interrupt::EdgeLow) {
            critical_section::with(|cs| *ENC_A_COUNTER.borrow(cs).borrow_mut() += 1);
            enc_a.clear_interrupt(gpio::Interrupt::EdgeLow);
        }
        if enc_b.interrupt_status(gpio::Interrupt::EdgeHigh) {
            critical_section::with(|cs| *ENC_B_COUNTER.borrow(cs).borrow_mut() += 1);
            enc_b.clear_interrupt(gpio::Interrupt::EdgeHigh);
        }
        if enc_b.interrupt_status(gpio::Interrupt::EdgeLow) {
            critical_section::with(|cs| *ENC_B_COUNTER.borrow(cs).borrow_mut() += 1);
            enc_b.clear_interrupt(gpio::Interrupt::EdgeLow);
        }
    }
}
