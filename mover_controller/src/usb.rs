use rp2040_hal::usb::UsbBus;
use usb_device::{
    bus::UsbBusAllocator,
    device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid},
};
use usbd_serial::SerialPort;

use crate::system;

pub struct UsbSerial {
    alloc: UsbBusAllocator<UsbBus>,
    pub serial: SerialPort<'static, UsbBus>,
    pub usb_dev: UsbDevice<'static, UsbBus>,
}

impl UsbSerial {
    pub fn new(mut system: system::System) -> Self {
        let alloc = UsbBusAllocator::new(UsbBus::new(
            system.pac.USBCTRL_REGS,
            system.pac.USBCTRL_DPRAM,
            system.clocks.usb_clock,
            true,
            &mut system.pac.RESETS,
        ));

        let mut serial = SerialPort::new(&alloc);
        let mut usb_dev = UsbDeviceBuilder::new(&alloc, UsbVidPid(0x16c0, 0x27dd))
            .strings(&[StringDescriptors::default()
                .manufacturer("tuxy")
                .product("mover")
                .serial_number("1")])
            .unwrap()
            .max_packet_size_0(64)
            .unwrap()
            .device_class(2)
            .build();

        Self {
            alloc,
            serial,
            usb_dev,
        }
    }
}
