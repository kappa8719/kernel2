use core::arch::naked_asm;

#[repr(packed)]
pub struct TrapFrame {
    pub ra: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub sp: usize,
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub fn exception_entrypoint() {
    naked_asm!(
        r#"
    .align 4
    
    csrw sscratch, sp
    addi sp, sp, -4 * 31
    sw ra,  4 * 0(sp)
    sw gp,  4 * 1(sp)
    sw tp,  4 * 2(sp)
    sw t0,  4 * 3(sp)
    sw t1,  4 * 4(sp)
    sw t2,  4 * 5(sp)
    sw t3,  4 * 6(sp)
    sw t4,  4 * 7(sp)
    sw t5,  4 * 8(sp)
    sw t6,  4 * 9(sp)
    sw a0,  4 * 10(sp)
    sw a1,  4 * 11(sp)
    sw a2,  4 * 12(sp)
    sw a3,  4 * 13(sp)
    sw a4,  4 * 14(sp)
    sw a5,  4 * 15(sp)
    sw a6,  4 * 16(sp)
    sw a7,  4 * 17(sp)
    sw s0,  4 * 18(sp)
    sw s1,  4 * 19(sp)
    sw s2,  4 * 20(sp)
    sw s3,  4 * 21(sp)
    sw s4,  4 * 22(sp)
    sw s5,  4 * 23(sp)
    sw s6,  4 * 24(sp)
    sw s7,  4 * 25(sp)
    sw s8,  4 * 26(sp)
    sw s9,  4 * 27(sp)
    sw s10, 4 * 28(sp)
    sw s11, 4 * 29(sp)

    csrr a0, sscratch
    sw a0, 4 * 30(sp)

    mv a0, sp
    call handle_trap

    lw ra,  4 * 0(sp)
    lw gp,  4 * 1(sp)
    lw tp,  4 * 2(sp)
    lw t0,  4 * 3(sp)
    lw t1,  4 * 4(sp)
    lw t2,  4 * 5(sp)
    lw t3,  4 * 6(sp)
    lw t4,  4 * 7(sp)
    lw t5,  4 * 8(sp)
    lw t6,  4 * 9(sp)
    lw a0,  4 * 10(sp)
    lw a1,  4 * 11(sp)
    lw a2,  4 * 12(sp)
    lw a3,  4 * 13(sp)
    lw a4,  4 * 14(sp)
    lw a5,  4 * 15(sp)
    lw a6,  4 * 16(sp)
    lw a7,  4 * 17(sp)
    lw s0,  4 * 18(sp)
    lw s1,  4 * 19(sp)
    lw s2,  4 * 20(sp)
    lw s3,  4 * 21(sp)
    lw s4,  4 * 22(sp)
    lw s5,  4 * 23(sp)
    lw s6,  4 * 24(sp)
    lw s7,  4 * 25(sp)
    lw s8,  4 * 26(sp)
    lw s9,  4 * 27(sp)
    lw s10, 4 * 28(sp)
    lw s11, 4 * 29(sp)
    lw sp,  4 * 30(sp)
    sret
    "#
    )
}

#[unsafe(no_mangle)]
fn handle_trap(frame: TrapFrame) {
    let scause = riscv::register::scause::read();
    let stval = riscv::register::stval::read();
    let sepc = riscv::register::sepc::read();

    panic!(
        "unexpected trap (scause={scause:?}, stval={stval}, sepc={sepc:#x})",
        scause = scause.cause(),
    );
}
