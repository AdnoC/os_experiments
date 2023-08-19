UNAME_S = $(shell uname -s)

QEMU = "qemu-system-aarch64"
HAS_QEMU := $(shell which ${QEMU} 2>/dev/null)

ifndef HAS_QEMU
	QEMU = "qemu-system-aarch64.exe"
endif
ifeq ($(UNAME_S), Linux)
	export CC = aarch64-linux-gnu-gcc
	export AR = aarch64-linux-gnu-ar
	export LD = aarch64-linux-gnu-ld
	export OCOPY = aarch64-linux-gnu-objcopy
	export GDB = aarch64-linux-gnu-gdb
	export CFLAGS = -march=armv8-a -Wall -O3 -nostdlib -nostartfiles -ffreestanding -mtune=cortex-a53
	export RUSTFLAGS = -C linker=${CC} -C target-cpu=cortex-a53 -C target-feature=+strict-align,+a53,+fp-armv8,+neon -C link-arg=-nostartfiles -C link-arg=-T./kernel8.ld

endif
ifeq ($(UNAME_S), Darwin)
	export CC = "aarch64-unknown-linux-gnu-gcc"
	export AR = "aarch64-unknown-linux-gnu-ar"
	export LD = "aarch64-unknown-linux-gnu-ld"
	export OCOPY = "aarch64-unknown-linux-gnu-objcopy"
	export GDB = "aarch64-unknown-linux-gnu-gcc"
	export CFLAGS = "-march=armv8-a -Wall -O3 -nostdlib -nostartfiles -ffreestanding -mtune=cortex-a53"
	export RUSTFLAGS = "-C linker=${CC} -C target-cpu=cortex-a53 -C target-feature=+strict-align,+a53,+fp-armv8,+neon -C link-arg=-nostartfiles -C link-arg=-T./kernel8.ld"
endif


.PHONY: build
build: target/aarch64-unknown-linux-gnu/release/os_experiments

.PHONY: qemu
qemu: target/kernel.img
	@echo "(Press Ctrl-A X to exit QEMU.)"
	${QEMU} -M raspi3b -nographic -kernel target/kernel.img -serial null -serial mon:stdio

target/kernel.img: target/aarch64-unknown-linux-gnu/release/os_experiments
	${OCOPY} -O binary ./target/aarch64-unknown-linux-gnu/release/os_experiments target/kernel.img

RUST_SRC = $(wildcard src/*.rs) $(wildcard src/**/*.rs)
target/aarch64-unknown-linux-gnu/release/os_experiments: ${RUST_SRC} build.rs kernel8.ld
	cargo build --release
