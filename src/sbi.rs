use core::arch::asm;
use core::fmt;
use core::fmt::Write;

pub struct SBIReturn {
    pub error: usize,
    pub value: usize,
}



pub fn sbi_call(
    mut arg0: usize,
    mut arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    fid: usize,
    eid: usize,
) -> SBIReturn {
    unsafe {
        asm!("ecall", inout("a0") arg0 => arg0, inout("a1") arg1 => arg1, in("a2") arg2, in("a3") arg3, in("a4") arg4, in("a5") arg5, in("a6") fid, in("a7") eid);
    }

    SBIReturn {
        error: arg0,
        value: arg1,
    }
}


pub fn put_char(ch: char) {
    sbi_call(ch as usize, 0, 0, 0, 0, 0, 0, 1);
}

pub struct SBIWriter;

impl Write for SBIWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for i in 0..s.len() {
            put_char(s.as_bytes()[i] as char);
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        core::fmt::Write::write_fmt(&mut $crate::sbi::SBIWriter, format_args!($($arg)*)).unwrap();
    }
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}