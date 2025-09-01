/// A Bus/Device/Function struct
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BusDeviceFunction(u16);

impl BusDeviceFunction {
    pub fn new(bus: u8, device: u8, function: u8) -> BusDeviceFunction {
        let bus = bus as u16;
        let device = device as u16;
        let function = function as u16;
        Self(bus << 8 | device << 5 | function)
    }

    pub fn bus(self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub fn device(self) -> u8 {
        ((self.0 >> 3) & 0b11111) as u8
    }

    pub fn function(self) -> u8 {
        (self.0 & 0b111) as u8
    }
}

pub fn configuration_read_word(base: usize, bdf: BusDeviceFunction, offset: u8) -> u16 {
    let bus = bdf.bus() as u32;
    let device = bdf.device() as u32;
    let function = bdf.function() as u32;
    let address = 0x80000000 | bus << 16 | device << 16 | function << 8 | (offset as u32) & 0xfc;

    let o = (base + 0xcf8) as *mut u32;
    unsafe { o.write_volatile(address) };

    let i = (base + 0xcfc) as *mut u32;
    let read = unsafe { i.read_volatile() };

    (read >> (((offset as u16) & 2) * 8) & 0xFFFF) as u16
}
