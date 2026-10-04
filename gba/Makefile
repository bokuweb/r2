# Build the GBA ROM: `make` (Linux) or `make baremetal`.
SYSROOT := $(shell rustc --print sysroot)
OBJCOPY := $(wildcard $(SYSROOT)/lib/rustlib/*/bin/llvm-objcopy)
ELF := target/armv4t-none-eabi/release/r2-gba

.PHONY: all baremetal clean FORCE

all: target/r2-gba.gba

baremetal: FORCE
	cargo build --release --features baremetal --target-dir target/baremetal
	$(OBJCOPY) -O binary target/baremetal/armv4t-none-eabi/release/r2-gba target/r2-gba-baremetal.gba

target/r2-gba.gba: FORCE
	cargo build --release
	$(OBJCOPY) -O binary $(ELF) $@

clean:
	cargo clean
