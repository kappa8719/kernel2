#!/bin/bash
set -xue

QEMU=qemu-system-riscv32

cargo build

$QEMU -machine virt -bios default -nographic -serial mon:stdio --no-reboot "$@" \
      -kernel ./target/riscv32i-unknown-none-elf/debug/kappa