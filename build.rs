// Copyright (C) 2020 Gilberto "jibi" Bertin <me@jibi.io>
//
// This file is part of hydrogen peroxyde.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

extern crate bindgen;

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo:rerun-if-changed=src/xsk/sys/wrapper.h");
    println!("cargo:rerun-if-changed=deps/libbpf/src/libbpf.so");
    println!("cargo:rerun-if-changed=kern/xsk_kern.c");
    println!("cargo:rerun-if-changed=kern/utils.h");

    println!(
        "cargo:rustc-link-search=native={}/deps/libbpf/src",
        manifest_dir
    );
    println!("cargo:rustc-link-lib=static=bpf");

    println!("cargo:rustc-link-lib=elf");
    println!("cargo:rustc-link-lib=z");

    let bindings = bindgen::Builder::default()
        .header("src/xsk/sys/wrapper.h")
        .clang_arg("-Ideps/libbpf/include/uapi")
        .clang_arg("-Ideps/libbpf/src")
        .generate()
        .expect("Unable to generate XSK bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    Command::new("make")
        .args(&["-C", "kern"])
        .status()
        .expect("Failed to build XSK kernel object");

    Command::new("make")
        .args(&["-C", "deps/libbpf/src"])
        .status()
        .expect("Failed to build libbpf");
}
