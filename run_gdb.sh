#!/bin/sh
aarch64-unknown-linux-gnu-gdb -ex "target remote :1234" ./target/aarch64-ruspiro/release/os_experiments
