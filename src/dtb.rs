use crate::println;
use core::arch::asm;
use fdt::Fdt;

#[derive(Debug, Clone)]
#[repr(C)]
struct FdtHeader {
    magic: u32,
    totalsize: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsvmap: u32,
    version: u32,
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_strings: u32,
    size_dt_struct: u32,
}

struct FdtMemoryReservationBlock {}

static mut FDT_ADDRESS: usize = 0;
static mut FDT: Option<Fdt> = None;

/// updates global fdt address variable
pub unsafe fn update_fdt_address() {
    unsafe { asm!("addi t3, a1, 0", out("t3") FDT_ADDRESS) };
}

/// prints some debug information about global fdt
#[allow(static_mut_refs)]
pub fn load_fdt() {
    unsafe { FDT = Fdt::from_ptr(FDT_ADDRESS as *const u8).ok() };
}

pub fn fdt() -> Fdt<'static> {
    unsafe { FDT.unwrap() }
}
