#![no_std]
#![no_main]
#![feature(fn_align)]
#![feature(naked_functions_rustic_abi)]
#![feature(range_into_bounds)]

mod arch;
mod dtb;
mod exceptions;
mod memory;
mod proc;
mod sbi;
mod util;

use crate::arch::rvc::context_switch;
use crate::proc::Proc;
use crate::util::memset;
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
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub unsafe extern "C" fn boot() {
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

static mut PROC_A: *mut Proc = core::ptr::null_mut();
static mut PROC_B: *mut Proc = core::ptr::null_mut();

fn delauy() {
    for _ in 0..3000000 {
        riscv::asm::nop();
    }
}

fn proc_a_entry() {
    println!("init proc a");

    loop {
        print!("A");
        let a = unsafe { &mut *PROC_A };
        let b = unsafe { &mut *PROC_B };
        
        unsafe {
            context_switch(
                &mut a.stack_pointer.0,
                &mut b.stack_pointer.0,
            )
        };
        delauy();
    }
}

fn proc_b_entry() {
    println!("init proc b");

    loop {
        print!("B");
        let a = unsafe { &mut *PROC_A };
        let b = unsafe { &mut *PROC_B };
        unsafe {
            context_switch(
                &mut b.stack_pointer.0,
                &mut a.stack_pointer.0,
            )
        };
        delauy();
    }
}

pub unsafe fn kernel_main() -> ! {
    println!("kernel is initializing");

    unsafe {
        println!("__kernel_base {:#x}", ld_variable!(__kernel_base, u8));
        println!("__stack_top {:#x}", ld_variable!(__stack_top, usize));
        println!("__bss {__bss:#x}");
        println!("__bss_end {__bss_end:#x}");

        memset(
            &__bss as *const u8 as *mut u8,
            0,
            (__bss_end - __bss) as usize,
        );
    }

    unsafe {
        riscv::register::stvec::write(Stvec::from_bits(exceptions::exception_entrypoint as usize))
    };

    dtb::load_fdt();
    memory::set_region_from_fdt();

    println!("kernel has been initialized");

    PROC_A = Proc::create(proc_a_entry as usize);
    PROC_B = Proc::create(proc_b_entry as usize);
    proc_a_entry();

    loop {
        riscv::asm::wfi();
    }
}

#[panic_handler]
pub fn panic_handler(info: &PanicInfo) -> ! {
    println!("kernel panicked: {info}");

    loop {}
}
