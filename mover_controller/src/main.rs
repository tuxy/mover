#![no_std]
#![no_main]

mod motor;
mod pid;
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
use crate::pid::PIDController;
use crate::system::System;
use crate::usb::UsbSerial;

const PWM_TOP: u16 = 65535;
const PPR: f32 = 465.0;

#[link_section = ".boot2"]
#[used]
static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

type EncoderPinA = gpio::Pin<gpio::bank0::Gpio8, gpio::FunctionSioInput, gpio::PullUp>;
type EncoderPinB = gpio::Pin<gpio::bank0::Gpio9, gpio::FunctionSioInput, gpio::PullUp>;
type EncoderPins = (EncoderPinB, EncoderPinA);

static ENCODER_PINS: Mutex<RefCell<Option<EncoderPins>>> = Mutex::new(RefCell::new(None));
static ENC_A_COUNTER: Mutex<RefCell<u32>> = Mutex::new(RefCell::new(0));
static ENC_B_COUNTER: Mutex<RefCell<u32>> = Mutex::new(RefCell::new(0));

pub struct State {
    pub stby: bool,
    pub speed_a: f32,
    pub speed_b: f32,
    pub target_a: f32,
    pub target_b: f32,
}

macro_rules! setup_motor {
    ($name:ident, $pwm:expr, $chan:ident, $ena_pin:expr, [$in1:expr, $in2:expr]) => {
        let pwm = &mut $pwm;
        pwm.default_config();
        pwm.set_top(PWM_TOP);
        pwm.enable();
        let en = &mut pwm.$chan;
        en.output_to($ena_pin);
        let pins: [ErasedOutputPin; 2] = [$in1.into_dyn_pin(), $in2.into_dyn_pin()];
        let mut $name = OpenMotorController::new(en, pins);
    };
}

fn apply_speed<P: embedded_hal::pwm::SetDutyCycle>(
    controller: &mut OpenMotorController<P>,
    value: f32,
) {
    controller.set_percentage(value.abs() as u16);
    if value >= 0.0 {
        controller.set_direction(MotorDirection::Forward);
    } else {
        controller.set_direction(MotorDirection::Reverse);
    }
}

fn write_str(terminal: &mut UsbSerial, s: &str) {
    let _ = terminal.write(s.as_bytes());
}

fn write_f32(terminal: &mut UsbSerial, buffer: &mut ryu::Buffer, value: f32) {
    let _ = terminal.write(buffer.format(value).as_bytes());
}

fn process_serial_cmd(
    line: &[u8],
    state: &mut State,
    pid: &mut PIDController,
    terminal: &mut UsbSerial,
) {
    if line.is_empty() {
        return;
    }

    if line == b"?" {
        let mut buf_a = ryu::Buffer::new();
        let mut buf_b = ryu::Buffer::new();
        write_str(terminal, "kp:");
        write_f32(terminal, &mut buf_a, pid.kp);
        write_str(terminal, " ki:");
        write_f32(terminal, &mut buf_b, pid.ki);
        write_str(terminal, " kd:");
        write_f32(terminal, &mut buf_a, pid.kd);
        write_str(terminal, " sa:");
        write_f32(terminal, &mut buf_b, state.speed_a);
        write_str(terminal, " sb:");
        write_f32(terminal, &mut buf_a, state.speed_b);
        write_str(terminal, " ta:");
        write_f32(terminal, &mut buf_b, state.target_a);
        write_str(terminal, " tb:");
        write_f32(terminal, &mut buf_a, state.target_b);
        let _ = terminal.write(b"\r\n");
        return;
    }

    if line.len() < 3 {
        return;
    }
    let sep = line[2];
    if sep != b' ' && sep != b'=' {
        return;
    }
    let cmd = &line[..2];
    let val = core::str::from_utf8(&line[3..])
        .ok()
        .and_then(|s| s.parse::<f32>().ok());
    let Some(val) = val else { return };

    match cmd {
        b"ta" => state.target_a = val,
        b"tb" => state.target_b = val,
        b"kp" => pid.kp = val,
        b"ki" => pid.ki = val,
        b"kd" => pid.kd = val,
        b"stby" => state.stby = !state.stby,
        _ => {}
    }
}

#[hal::entry]
fn main() -> ! {
    let mut system = System::init();

    let mut stby_pin = system.pins.gpio7.into_push_pull_output();
    stby_pin.set_high();

    // Steal PAC for peripherals consumed after System::init
    let mut pac = unsafe { hal::pac::Peripherals::steal() };

    // Encoder inputs
    let enc_a = system.pins.gpio9.into_pull_up_input();
    let enc_b = system.pins.gpio8.into_pull_up_input();

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
        pwm_slices.pwm6,
        channel_b,
        system.pins.gpio13,
        [
            system.pins.gpio14.into_push_pull_output(),
            system.pins.gpio15.into_push_pull_output()
        ]
    );

    setup_motor!(
        motorb_controller,
        pwm_slices.pwm5,
        channel_a,
        system.pins.gpio10,
        [
            system.pins.gpio11.into_push_pull_output(),
            system.pins.gpio12.into_push_pull_output()
        ]
    );

    // Main loop
    let mut led_pin = system.pins.gpio25.into_push_pull_output();
    let mut state = State {
        stby: false,
        speed_a: 0.0,
        speed_b: 0.0,
        target_a: 0.0,
        target_b: 0.0,
    };
    let mut line_buf = [0u8; 32];
    let mut line_idx = 0;
    let mut read_buf = [0u8; 16];

    let mut pid = PIDController::init(1000.0, 800.0, 0.0);
    let mut last_time: u64 = 0;

    loop {
        terminal.poll();

        // Read and process serial input
        if let Ok(n) = terminal.read(&mut read_buf) {
            for &b in &read_buf[..n] {
                if b == b'\n' || b == b'\r' {
                    if line_idx > 0 {
                        process_serial_cmd(
                            &line_buf[..line_idx],
                            &mut state,
                            &mut pid,
                            &mut terminal,
                        );
                    }
                    line_idx = 0;
                } else if line_idx < line_buf.len() {
                    line_buf[line_idx] = b;
                    line_idx += 1;
                }
            }
        }

        let (enc_a, enc_b) = critical_section::with(|cs| {
            let a = *ENC_A_COUNTER.borrow(cs).borrow();
            let b = *ENC_B_COUNTER.borrow(cs).borrow();
            *ENC_A_COUNTER.borrow(cs).borrow_mut() = 0;
            *ENC_B_COUNTER.borrow(cs).borrow_mut() = 0;
            (a, b)
        });

        if last_time == 0 {
            last_time = system.timer.get_counter().ticks();
        }
        let current_time = system.timer.get_counter().ticks();
        let delta_us = (current_time - last_time) as f32;
        last_time = current_time;

        let delta_s = if delta_us > 0.0 {
            delta_us / 1_000_000.0
        } else {
            0.01
        };
        state.speed_b = (enc_a as f32) / (PPR * 2.0) * 60.0 / delta_s;
        state.speed_a = (enc_b as f32) / (PPR * 2.0) * 60.0 / delta_s;

        match state.stby {
            true => stby_pin.set_high(),
            false => stby_pin.set_low(),
        };

        let a = pid.compute_iteration(state.target_a.abs(), state.speed_a, delta_us);
        let b = pid.compute_iteration(state.target_b.abs(), state.speed_b, delta_us);

        apply_speed(
            &mut motora_controller,
            a * (state.target_a / state.target_a.abs()),
        );
        apply_speed(
            &mut motorb_controller,
            b * (state.target_b / state.target_b.abs()),
        );

        if terminal.is_configured() {
            let _ = led_pin.set_high();
        } else {
            let _ = led_pin.set_low();
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

    if let Some((enc_b, enc_a)) = PINS {
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
