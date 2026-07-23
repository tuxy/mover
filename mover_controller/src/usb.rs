use rp2040_hal::usb::UsbBus;
use usb_device::{
    bus::UsbBusAllocator,
    device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid},
    prelude::UsbDeviceState,
    UsbError,
};
use usbd_serial::SerialPort;

fn make_alloc_static(alloc: UsbBusAllocator<UsbBus>) -> &'static UsbBusAllocator<UsbBus> {
    #[allow(static_mut_refs)]
    unsafe {
        static mut ALLOC: Option<UsbBusAllocator<UsbBus>> = None;
        ALLOC = Some(alloc);
        ALLOC.as_ref().unwrap()
    }
}

pub struct UsbSerial {
    pub serial: SerialPort<'static, UsbBus>,
    pub usb_dev: UsbDevice<'static, UsbBus>,
}

impl UsbSerial {
    pub fn new(
        regs: rp2040_hal::pac::USBCTRL_REGS,
        dpram: rp2040_hal::pac::USBCTRL_DPRAM,
        usb_clock: rp2040_hal::clocks::UsbClock,
        resets: &mut rp2040_hal::pac::RESETS,
    ) -> Self {
        let alloc = make_alloc_static(UsbBusAllocator::new(UsbBus::new(
            regs, dpram, usb_clock, true, resets,
        )));

        let serial = SerialPort::new(alloc);
        let usb_dev = UsbDeviceBuilder::new(alloc, UsbVidPid(0x16c0, 0x27dd))
            .strings(&[StringDescriptors::default()
                .manufacturer("tuxy")
                .product("mover")
                .serial_number("1")])
            .unwrap()
            .max_packet_size_0(64)
            .unwrap()
            .device_class(2)
            .build();

        Self { serial, usb_dev }
    }

    pub fn poll(&mut self) {
        self.usb_dev.poll(&mut [&mut self.serial]);
    }

    pub fn write(&mut self, data: &[u8]) -> core::result::Result<usize, UsbError> {
        self.serial.write(data)
    }

    pub fn is_configured(&self) -> bool {
        self.usb_dev.state() == UsbDeviceState::Configured
    }
}
