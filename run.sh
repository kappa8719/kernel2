#!/bin/bash
set -xue

QEMU=qemu-system-riscv32

cargo build

$QEMU -machine virt -bios default -nographic -serial mon:stdio --no-reboot "$@" \
  -drive id=drive0,file=virtio-blk-sample,format=raw,if=none \
  -device virtio-blk-device,drive=drive0,bus=virtio-mmio-bus.0 \
  -drive file=fat:rw:.disk/ \
  -kernel ./target/riscv32i-unknown-none-elf/debug/kappa

