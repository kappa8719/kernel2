#![no_std]
#![no_main]
#![feature(fn_align)]
#![feature(naked_functions_rustic_abi)]
#![feature(range_into_bounds)]

extern crate alloc;

mod allocator;
mod arch;
mod dtb;
mod exceptions;
mod filesystem;
mod memory;
mod paging;
mod proc;
mod proc2;
mod sbi;
mod util;

use crate::allocator::BuddyAllocator;
use crate::filesystem::virtio;
use alloc::string::String;
use alloc::vec;
use core::arch::asm;
use core::panic::PanicInfo;
use riscv::register::stvec::Stvec;

#[macro_export]
macro_rules! ld_variable {
    ($var: expr, $t: ty) => {
        &$var as *const $t as usize
    };
}

unsafe extern "C" {
    pub static __kernel_base: u8;
    pub static __bss: u8;
    pub static __bss_end: u8;
    pub static __stack_top: usize;
    pub static __kernel_heap: u8;
    pub static __kernel_heap_end: u8;
}

static mut KERNEL_HEAP: *mut u8 = core::ptr::null_mut();

unsafe fn kernel_heap_init() {
    unsafe { KERNEL_HEAP = ld_variable!(__kernel_heap, u8) as *mut u8 }
}

/// Reserve kernel heap of size and increase the pointer
///
/// # Returns
/// This function returns the start of reserved heap range as `*mut usize` or null pointer when
/// available heap is less then the requested size
unsafe fn kernel_heap_reserve(size: usize) -> *mut u8 {
    unsafe {
        if kernel_heap_available() < size {
            return core::ptr::null_mut();
        }

        let ptr = KERNEL_HEAP;
        KERNEL_HEAP = KERNEL_HEAP.add(size);
        ptr
    }
}

fn kernel_heap_available() -> usize {
    unsafe { ld_variable!(__kernel_heap_end, u8) - (KERNEL_HEAP as usize) }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
unsafe extern "C" fn boot() {
    unsafe {
        dtb::update_fdt_address();

        // reset stack top and jump to kernal_main
        asm!(
        "mv sp, {stack_top}
            j {kernel_main}",
        stack_top = in(reg) &__stack_top,
        kernel_main = sym kernel_main,
        );
    }
}

unsafe fn kernel_main() -> ! {
    unsafe {
        println!("kernel is initializing");

        println!("__kernel_base {:#x}", ld_variable!(__kernel_base, u8));
        println!("__stack_top {:#x}", ld_variable!(__stack_top, usize));
        println!("__bss {:#x}", ld_variable!(__bss, u8));
        println!("__bss_end {:#x}", ld_variable!(__bss_end, u8));
        println!("__kernel_heap {:#x}", ld_variable!(__kernel_heap, u8));
        println!(
            "__kernel_heap_end {:#x}",
            ld_variable!(__kernel_heap_end, u8)
        );

        // init bss
        core::ptr::write_bytes(
            &__bss as *const u8 as *mut u8,
            0,
            (__bss_end - __bss) as usize,
        );

        // init kernel heap
        kernel_heap_init();

        // initialize trap handler
        riscv::register::stvec::write(Stvec::from_bits(exceptions::exception_entrypoint as usize));

        dtb::load_fdt();
        memory::set_region_from_fdt();

        // init allocator
        let required_heap = BuddyAllocator::get_required_heap(memory::get_region().size);
        let allocator_heap = kernel_heap_reserve(required_heap);
        if allocator_heap.is_null() {
            panic!(
                "kernel heap is not enough to initialize global allocator ({}/{})",
                kernel_heap_available(),
                required_heap
            );
        }
        allocator::initialize_global(
            memory::get_region().clone(),
            &mut *core::ptr::slice_from_raw_parts_mut(allocator_heap, required_heap),
        );

        virtio::initialize();

        println!("kernel has been initialized");
        println!("kernel heap: {} available", kernel_heap_available());
        println!("global heap region {:x?}", memory::get_region());

        loop {
            riscv::asm::wfi();
        }
    }
}

#[panic_handler]
pub fn panic_handler(info: &PanicInfo) -> ! {
    println!("kernel panicked: {info}");

    loop {}
}
