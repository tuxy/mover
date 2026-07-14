#![no_std]
#![no_main]

mod motor;
mod usb;

use core::cell::RefCell;
use critical_section::Mutex;
use embedded_hal::delay::DelayNs;
use hal::pac;
use hal::pac::interrupt;
use panic_halt as _;
use rp2040_hal::{self as hal, gpio};

use crate::motor::{ErasedOutputPin, MotorDirection, OpenMotorController};

const XTAL_FREQ_HZ: u32 = 12_000_000;
const PWM_TOP: u16 = 65535;

#[link_section = ".boot2"]
#[used]
static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

// ── Encoder interrupt types ──────────────────────────────────────────────────
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
    // Pico setup (should not modify)
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let mut timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // ── Encoder input setup ──────────────────────────────────────────────────
    let enc_a = pins.gpio8.into_pull_up_input();
    let enc_b = pins.gpio9.into_pull_up_input();

    enc_a.set_interrupt_enabled(gpio::Interrupt::EdgeHigh, true);
    enc_a.set_interrupt_enabled(gpio::Interrupt::EdgeLow, true);
    enc_b.set_interrupt_enabled(gpio::Interrupt::EdgeHigh, true);
    enc_b.set_interrupt_enabled(gpio::Interrupt::EdgeLow, true);

    critical_section::with(|cs| {
        ENCODER_PINS.borrow(cs).replace(Some((enc_a, enc_b)));
    });

    unsafe {
        pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0);
    }

    // ── USB serial setup ─────────────────────────────────────────────────────
    usb::init(pac.USBCTRL_REGS, pac.USBCTRL_DPRAM, clocks.usb_clock, &mut pac.RESETS);

    unsafe {
        pac::NVIC::unmask(pac::Interrupt::USBCTRL_IRQ);
    }

    // ── PWM & open-loop motor setup ──────────────────────────────────────────
    let mut pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);

    setup_motor!(
        motora_controller,
        pwm_slices.pwm5,
        channel_a,
        pins.gpio10,
        [
            pins.gpio11.into_push_pull_output(),
            pins.gpio12.into_push_pull_output()
        ]
    );

    setup_motor!(
        motorb_controller,
        pwm_slices.pwm6,
        channel_b,
        pins.gpio13,
        [
            pins.gpio14.into_push_pull_output(),
            pins.gpio15.into_push_pull_output()
        ]
    );

    // ── Wait for USB enumeration ─────────────────────────────────────────────
    for _ in 0..5000 {
        usb::poll();
        if usb::is_configured() {
            break;
        }
        timer.delay_us(1000);
    }

    // ── Main loop ────────────────────────────────────────────────────────────
    let mut main_timer: u32 = 0;
    loop {
        usb::poll();

        motora_controller.set_percentage(65535 / 2);
        motora_controller.set_direction(MotorDirection::Forward);
        motorb_controller.set_percentage(65535 / 2);
        motorb_controller.set_direction(MotorDirection::Reverse);

        if main_timer % 50 == 0 {
            let _ = usb::write("hello\n");
        }

        main_timer += 1;
        timer.delay_ms(10);
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
            let _ = critical_section::with(|cs| *ENC_A_COUNTER.borrow(cs).borrow_mut() += 1);
            enc_a.clear_interrupt(gpio::Interrupt::EdgeHigh);
        }
        if enc_a.interrupt_status(gpio::Interrupt::EdgeLow) {
            let _ = critical_section::with(|cs| *ENC_A_COUNTER.borrow(cs).borrow_mut() += 1);
            enc_a.clear_interrupt(gpio::Interrupt::EdgeLow);
        }
        if enc_b.interrupt_status(gpio::Interrupt::EdgeHigh) {
            let _ = critical_section::with(|cs| *ENC_B_COUNTER.borrow(cs).borrow_mut() += 1);
            enc_b.clear_interrupt(gpio::Interrupt::EdgeHigh);
        }
        if enc_b.interrupt_status(gpio::Interrupt::EdgeLow) {
            let _ = critical_section::with(|cs| *ENC_B_COUNTER.borrow(cs).borrow_mut() += 1);
            enc_b.clear_interrupt(gpio::Interrupt::EdgeLow);
        }
    }
}

#[interrupt]
fn USBCTRL_IRQ() {
    usb::poll();
}
