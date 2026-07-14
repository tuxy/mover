use core::cell::RefCell;
use critical_section::Mutex;
use rp2040_hal::usb::UsbBus;
use usb_device::{
    bus::UsbBusAllocator,
    device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid},
    prelude::UsbDeviceState,
    Result, UsbError,
};
use usbd_serial::SerialPort;

struct UsbState {
    serial: SerialPort<'static, UsbBus>,
    device: UsbDevice<'static, UsbBus>,
}

unsafe impl Send for UsbState {}

fn make_alloc_static(
    alloc: UsbBusAllocator<UsbBus>,
) -> &'static UsbBusAllocator<UsbBus> {
    #[allow(static_mut_refs)]
    unsafe {
        static mut ALLOC: Option<UsbBusAllocator<UsbBus>> = None;
        ALLOC = Some(alloc);
        ALLOC.as_ref().unwrap()
    }
}

static USB_STATE: Mutex<RefCell<Option<UsbState>>> = Mutex::new(RefCell::new(None));

pub fn init(
    regs: rp2040_hal::pac::USBCTRL_REGS,
    dpram: rp2040_hal::pac::USBCTRL_DPRAM,
    usb_clock: rp2040_hal::clocks::UsbClock,
    resets: &mut rp2040_hal::pac::RESETS,
) {
    let alloc = make_alloc_static(UsbBusAllocator::new(UsbBus::new(
        regs, dpram, usb_clock, true, resets,
    )));

    let serial = SerialPort::new(alloc);
    let usb_dev = UsbDeviceBuilder::new(alloc, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("Fake company")
            .product("Serial port")
            .serial_number("TEST")])
        .unwrap()
        .max_packet_size_0(64)
        .unwrap()
        .device_class(2)
        .build();

    critical_section::with(|cs| {
        USB_STATE.borrow(cs).replace(Some(UsbState {
            serial,
            device: usb_dev,
        }));
    });
}

pub fn poll() {
    critical_section::with(|cs| {
        if let Some(state) = USB_STATE.borrow(cs).borrow_mut().as_mut() {
            state.device.poll(&mut [&mut state.serial]);
        }
    });
}

pub fn write(data: &str) -> Result<usize> {
    critical_section::with(|cs| {
        if let Some(state) = USB_STATE.borrow(cs).borrow_mut().as_mut() {
            state.serial.write(data.as_bytes())
        } else {
            Err(UsbError::InvalidState)
        }
    })
}

pub fn is_configured() -> bool {
    critical_section::with(|cs| {
        USB_STATE
            .borrow(cs)
            .borrow()
            .as_ref()
            .map_or(false, |s| s.device.state() == UsbDeviceState::Configured)
    })
}
