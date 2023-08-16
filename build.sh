#!/bin/sh
# cargo build --target thumbv7em-none-eabihf
# cargo build --target x86_64-blog_os.json
cargo bootimage --target x86_64-blog_os.json
