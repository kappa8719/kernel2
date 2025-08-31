use core::{alloc::Layout, mem::MaybeUninit};

use alloc::{alloc::alloc, boxed::Box};

use crate::println;

const SECTOR_SIZE: usize = 512;
const VIRTQ_ENTRY_NUM: usize = 16;
const VIRTIO_DEVICE_BLK: usize = 2;
const VIRTIO_BLK_PADDR: usize = 0x10001000;
const VIRTIO_REG_MAGIC: usize = 0x00;
const VIRTIO_REG_VERSION: usize = 0x04;
const VIRTIO_REG_DEVICE_ID: usize = 0x08;
const VIRTIO_REG_QUEUE_SEL: usize = 0x30;
const VIRTIO_REG_QUEUE_NUM_MAX: usize = 0x34;
const VIRTIO_REG_QUEUE_NUM: usize = 0x38;
const VIRTIO_REG_QUEUE_ALIGN: usize = 0x3c;
const VIRTIO_REG_QUEUE_PFN: usize = 0x40;
const VIRTIO_REG_QUEUE_READY: usize = 0x44;
const VIRTIO_REG_QUEUE_NOTIFY: usize = 0x50;
const VIRTIO_REG_DEVICE_STATUS: usize = 0x70;
const VIRTIO_REG_DEVICE_CONFIG: usize = 0x100;
const VIRTIO_STATUS_ACK: usize = 1;
const VIRTIO_STATUS_DRIVER: usize = 2;
const VIRTIO_STATUS_DRIVER_OK: usize = 4;
const VIRTIO_STATUS_FEAT_OK: usize = 8;
const VIRTQ_DESC_F_NEXT: usize = 1;
const VIRTQ_DESC_F_WRITE: usize = 2;
const VIRTQ_AVAIL_F_NO_INTERRUPT: usize = 1;
const VIRTIO_BLK_T_IN: usize = 0;
const VIRTIO_BLK_T_OUT: usize = 1;

#[repr(packed)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[repr(packed)]
struct VirtqAvailable {
    flags: u16,
    index: u16,
    ring: [u16; VIRTQ_ENTRY_NUM],
}

#[repr(packed)]
struct VirtqUsedEntry {
    id: u32,
    len: u32,
}

#[repr(packed)]
struct VirtqUsed {
    flags: u16,
    index: u16,
    ring: [VirtqUsedEntry; VIRTQ_ENTRY_NUM],
}

#[repr(packed)]
struct Virtq {
    descs: [VirtqDesc; VIRTQ_ENTRY_NUM],
    available: VirtqAvailable,
    used: VirtqUsed,
    queue_index: usize,
    used_index: *mut usize,
    last_used_index: usize,
}

impl Virtq {
    fn new(index: usize) -> *mut Self {
        let virtq = unsafe { alloc(Layout::new::<Virtq>()) } as *mut Virtq;
        mmio_write_u32(VIRTIO_REG_QUEUE_SEL, index as u32);
        mmio_write_u32(VIRTIO_REG_QUEUE_NUM, VIRTQ_ENTRY_NUM as u32);
        mmio_write_u32(VIRTIO_REG_QUEUE_ALIGN, 0);
        mmio_write_u32(VIRTIO_REG_QUEUE_PFN, virtq.addr() as u32);

        virtq
    }
}

#[repr(packed)]
struct VirtqBlkRequest {
    t: u32,
    reserved: u32,
    sector: u64,
    data: [u8; 512],
    status: u8,
}

fn mmio_read_u32(offset: usize) -> u32 {
    let ptr = (VIRTIO_BLK_PADDR + offset) as *mut u32;
    unsafe { ptr.read_volatile() }
}

fn mmio_read_u64(offset: usize) -> u64 {
    let ptr = (VIRTIO_BLK_PADDR + offset) as *mut u64;
    unsafe { ptr.read_volatile() }
}

fn mmio_write_u32(offset: usize, value: u32) {
    let ptr = (VIRTIO_BLK_PADDR + offset) as *mut u32;
    unsafe {
        ptr.write_volatile(value);
    }
}

fn mmio_write_u64(offset: usize, value: u64) {
    let ptr = (VIRTIO_BLK_PADDR + offset) as *mut u64;
    unsafe {
        ptr.write_volatile(value);
    }
}

fn mmio_fetch_and_or_u32(offset: usize, value: u32) {
    mmio_write_u32(offset, mmio_read_u32(offset) | value);
}
pub fn initialize() {
    if mmio_read_u32(VIRTIO_REG_MAGIC) != 0x74726976 {
        panic!("invalid magic value");
    }
    if mmio_read_u32(VIRTIO_REG_VERSION) != 1 {
        panic!("invalid version")
    }
    if mmio_read_u32(VIRTIO_REG_DEVICE_ID) != VIRTIO_DEVICE_BLK as u32 {
        panic!("invalid device id");
    }

    mmio_write_u32(VIRTIO_REG_DEVICE_STATUS, 0);
    mmio_fetch_and_or_u32(VIRTIO_REG_DEVICE_STATUS, VIRTIO_STATUS_ACK as u32);
    mmio_fetch_and_or_u32(VIRTIO_REG_DEVICE_STATUS, VIRTIO_STATUS_DRIVER as u32);
    mmio_fetch_and_or_u32(VIRTIO_REG_DEVICE_STATUS, VIRTIO_STATUS_FEAT_OK as u32);
    let virtq = Virtq::new(0);
    mmio_write_u32(VIRTIO_REG_DEVICE_STATUS, VIRTIO_STATUS_DRIVER_OK as u32);

    println!("initialized virtio-blk");
    let capacity = mmio_read_u64(VIRTIO_REG_DEVICE_CONFIG) * SECTOR_SIZE as u64;
    println!("virtio-blk: capacity {capacity}");
}
