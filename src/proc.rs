use crate::memory::{PAGE_SIZE, PAddr, PageFlag, VAddr, map_page_sv32, map_page_to_heap};
use crate::{__kernel_base, __stack_top, ld_variable};
use core::arch::asm;
use core::ops::Sub;
use macros::repeat;

const MAX_PROCESSES: usize = 8;

/// workaround for [https://github.com/rust-lang/rust/issues/44796]
const PROC_INIT: Proc = Proc::placeholder();
static mut PROCS: [Proc; MAX_PROCESSES] = [PROC_INIT; MAX_PROCESSES];

#[derive(PartialOrd, PartialEq, Ord, Eq, Copy, Clone, Debug)]
pub enum ProcState {
    Empty,
    Loaded,
}

pub struct Proc {
    pub pid: usize,
    state: ProcState,
    pub stack_pointer: VAddr,
    page_table: PAddr,
    stack: [u8; 8192],
}

impl Proc {
    pub const fn placeholder() -> Self {
        Self {
            pid: 0,
            state: ProcState::Empty,
            stack_pointer: VAddr::zero(),
            page_table: PAddr::zero(),
            stack: [0; 8192],
        }
    }
}

impl Proc {
    pub fn create(entrypoint: usize) -> &'static mut Proc {
        static mut PID_NEXT: usize = 0;

        let mut available: Option<&mut Proc> = None;
        #[allow(static_mut_refs)]
        for proc in unsafe { &mut PROCS } {
            if proc.state == ProcState::Empty {
                available = Some(proc);
            }
        }

        let Some(available) = available else {
            panic!("failed to create new process: reached max")
        };

        let stack_pointer = unsafe {
            available
                .stack
                .as_mut_ptr().add(size_of_val(&available.stack)) as *mut usize
        };
        unsafe {
            // s11 - s0 (12 writes)
            repeat!(12 as n, { *stack_pointer.sub(n + 1) = 0 });
            *stack_pointer.sub(13) = entrypoint; // ra
        }

        let page_table = crate::memory::allocate(1);

        map_page_to_heap(
            page_table,
            PageFlag::ReadWriteExecute,
            map_page_sv32,
        );
        // map kernel static memories
        for paddr in unsafe { ld_variable!(__kernel_base, u8)..ld_variable!(__stack_top, usize) }
            .step_by(PAGE_SIZE)
        {
            // println!("map {paddr:#x}");
            map_page_sv32(
                page_table,
                VAddr(paddr),
                PAddr(paddr),
                PageFlag::ReadWriteExecute,
            );
        }

        unsafe {
            PID_NEXT += 1;
        }
        available.pid = unsafe { PID_NEXT };
        available.stack_pointer = VAddr(unsafe { stack_pointer.sub(13) } as usize);
        available.state = ProcState::Loaded;
        available.page_table = page_table;

        available
    }

    #[inline(always)]
    pub unsafe fn switch_context(previous: &mut Proc, next: &mut Proc) {
        unsafe {
            let satp = riscv::register::satp::Mode::Sv32.into_usize() << 31
                | (next.page_table.addr() / PAGE_SIZE);
            let sscratch = (&mut next.stack as *mut [u8] as *mut u8)
                .add(size_of_val(&next.stack)) as *mut u32;

            asm!(
                "
                    sfence.vma
                    csrw satp, {satp}
                    sfence.vma
                    csrw sscratch, {sscratch}
                ",
                satp = in(reg) satp,
                sscratch = in(reg) sscratch
            );

            crate::arch::rvc::context_switch(
                &mut previous.stack_pointer.0,
                &mut next.stack_pointer.0,
            );
        }
    }
}
