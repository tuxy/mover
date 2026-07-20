use rp2040_hal::{clocks::ClocksManager, gpio::Pins, pac::Peripherals, Sio, Timer, Watchdog};

use core::cell::RefCell;
use critical_section::Mutex;
use embedded_hal::delay::DelayNs;
use hal::pac;
use hal::pac::interrupt;
use panic_halt as _;
use rp2040_hal::usb::UsbBus;
use rp2040_hal::{self as hal, gpio};
use usb_device::device::{StringDescriptors, UsbVidPid};
use usb_device::{bus::UsbBusAllocator, device::UsbDeviceBuilder};
use usbd_serial::SerialPort;

const XTAL_FREQ_HZ: u32 = 12_000_000;

pub struct System {
    pub pac: Peripherals,
    pub clocks: ClocksManager,
    pub timer: Timer,
    pub pins: Pins,
    sio: Sio,
    watchdog: Watchdog,
}

impl System {
    pub fn init() -> System {
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

        Self {
            pac,
            clocks,
            timer,
            pins,
            sio,
            watchdog,
        }
    }
}
