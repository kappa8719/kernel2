use crate::memory::VAddr;
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
    stack: [u8; 8192]
}

impl Proc {
    pub const fn placeholder() -> Self {
        Self {
            pid: 0,
            state: ProcState::Empty,
            stack_pointer: VAddr::zero(),
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
                .as_mut_ptr()
                .offset(size_of_val(&(*available).stack) as isize) as *mut usize
        };
        unsafe {
            // s11 - s0 (12 writes)
            repeat!(12 as n, { *stack_pointer.sub(n + 1) = 0 });
            *stack_pointer.sub(13) = entrypoint; // ra
        }

        unsafe {
            PID_NEXT += 1;
        }
        available.pid = unsafe { PID_NEXT };
        available.stack_pointer = VAddr(unsafe { stack_pointer.sub(13) } as usize);
        available.state = ProcState::Loaded;

        available
    }
}
