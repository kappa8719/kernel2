#!/bin/bash
set -xue

gdb-multiarch ./target/riscv32i-unknown-none-elf/debug/kappa -q -ex 'target remote :1234'