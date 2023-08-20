UNAME_S = $(shell uname -s)

QEMU = qemu-system-aarch64
HAS_QEMU := $(shell which ${QEMU} 2>/dev/null)

ifndef HAS_QEMU
	QEMU = qemu-system-aarch64.exe
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
	export CC = aarch64-unknown-linux-gnu-gcc
	export AR = aarch64-unknown-linux-gnu-ar
	export LD = aarch64-unknown-linux-gnu-ld
	export OCOPY = aarch64-unknown-linux-gnu-objcopy
	export GDB = aarch64-unknown-linux-gnu-gdb
	export CFLAGS = -march=armv8-a -Wall -O3 -nostdlib -nostartfiles -ffreestanding -mtune=cortex-a53
	export RUSTFLAGS = -C linker=${CC} -C target-cpu=cortex-a53 -C target-feature=+strict-align,+a53,+fp-armv8,+neon -C link-arg=-nostartfiles -C link-arg=-T./kernel8.ld
endif

DO_RELEASE = false
RELEASE_PATH = target/aarch64-unknown-linux-gnu/release
DEBUG_PATH = target/aarch64-unknown-linux-gnu/debug
ifeq ($(DO_RELEASE), true)
	RELEASE_FLAG = --release
	ELF_PATH = ${RELEASE_PATH}/os_experiments
	BUILD_DIR = ${RELEASE_PATH}
else
	RELEASE_FLAG =
	ELF_PATH = ${DEBUG_PATH}/os_experiments
	BUILD_DIR = ${DEBUG_PATH}
endif


.PHONY: build
build: ${ELF_PATH}

.PHONY: clean
clean:
	rm -f ${ELF_PATH}
	rm -f ${BUILD_DIR}/.cargo-lock

.PHONY: qemu
qemu: target/kernel.img
	@echo "(Press Ctrl-A X to exit QEMU.)"
	${QEMU} -M raspi3b -kernel target/kernel.img -serial null -serial mon:stdio

.PHONY: qemu-gdb
qemu-gdb: target/kernel.img
	@echo "(Press Ctrl-A X to exit QEMU.)"
	${QEMU} -M raspi3b -s -S -serial null -serial mon:stdio -kernel target/kernel.img

.PHONY: gdb
gdb:
	${GDB} -ex "target remote :1234" ${ELF_PATH}

target/kernel.img: ${ELF_PATH}
	${OCOPY} -O binary ${ELF_PATH} target/kernel.img

RUST_SRC = $(wildcard src/*.rs) $(wildcard src/**/*.rs) build.rs src/boot.s
${ELF_PATH}: ${RUST_SRC} build.rs kernel8.ld
	cargo build ${RELEASE_FLAG}
