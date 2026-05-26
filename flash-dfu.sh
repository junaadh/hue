#!/usr/bin/env bash
set -e

ELF="$1"
BIN="${ELF%.elf}.bin"

arm-none-eabi-objcopy -O binary "$ELF" "$BIN"

dfu-util -d 0483:df11 -a 0 -s 0x08000000:leave -D "$BIN"
