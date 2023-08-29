# NOTE: I have a Windows machine and a macbook. I only access Linux via the WSL.
# Thus any "Linux"-specific config stuff is for the Windows machine.

UNAME_S = $(shell uname -s)

QEMU = qemu-system-aarch64
HAS_QEMU := $(shell which ${QEMU} 2>/dev/null)

ifndef HAS_QEMU
	QEMU = qemu-system-aarch64.exe
endif
ifeq ($(UNAME_S), Linux)
	export GDB = gdb-multiarch
# 	# WSL can't access the Windows-based qemu through localhost.
# 	# We have to find the actual IP address of the host.
	export GDB_HOST = $(shell dig +short $(shell hostname).local | awk '{print; exit}')
#
endif
ifeq ($(UNAME_S), Darwin)
	export GDB = aarch64-unknown-linux-gnu-gdb
endif

export OCOPY = cargo-objcopy
DO_RELEASE = false
RELEASE_PATH = target/aarch64-unknown-none-softfloat/release
DEBUG_PATH = target/aarch64-unknown-none-softfloat/debug
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

.PHONY: lint-fix
lint-fix:
	cargo fix --target aarch64-unknown-linux-gnu ${RELEASE_FLAG} --allow-dirty


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
	${GDB} -ex "target remote ${GDB_HOST}:1234" ${ELF_PATH}

target/kernel.img: ${ELF_PATH}
	${OCOPY} ${RELEASE_FLAG} -- -O binary target/kernel.img

RUST_SRC = $(wildcard src/*.rs) $(wildcard src/**/*.rs)
ASM_SRC = $(wildcard src/*.s) $(wildcard src/**/*.s)
${ELF_PATH}: ${RUST_SRC} ${ASM_SRC} build.rs kernel8.ld
	cargo build -Z build-std=core,compiler_builtins -Z build-std-features=compiler-builtins-mem ${RELEASE_FLAG}
