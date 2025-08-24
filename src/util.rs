#[unsafe(no_mangle)]
#[inline(never)]
pub fn memset(buf: *mut u8, c: u8, n: usize) -> *mut u8 {
    let mut i = n;
    while i > 0 {
        i -= 1;
        let p = (buf as usize + i) as *mut u8;
        unsafe {
            *p = c;
        }
    }

    buf
}