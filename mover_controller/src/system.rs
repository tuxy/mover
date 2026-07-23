use rp2040_hal::{clocks::ClocksManager, gpio::Pins, pac::Peripherals, Sio, Timer, Watchdog};

const XTAL_FREQ_HZ: u32 = 12_000_000;

pub struct System {
    pub clocks: ClocksManager,
    pub timer: Timer,
    pub pins: Pins,
}

impl System {
    pub fn init() -> Self {
        let mut pac = Peripherals::take().unwrap();
        let mut watchdog = Watchdog::new(pac.WATCHDOG);

        let clocks = rp2040_hal::clocks::init_clocks_and_plls(
            XTAL_FREQ_HZ,
            pac.XOSC,
            pac.CLOCKS,
            pac.PLL_SYS,
            pac.PLL_USB,
            &mut pac.RESETS,
            &mut watchdog,
        )
        .unwrap();

        let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

        let sio = Sio::new(pac.SIO);
        let pins = Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );

        Self {
            clocks,
            timer,
            pins,
        }
    }
}
