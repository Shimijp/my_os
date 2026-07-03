# my_os

A small hobby **x86-64 operating system kernel** written in Rust. It boots via UEFI
(using the [`bootloader`](https://crates.io/crates/bootloader) `0.11` crate), draws text
directly to the UEFI framebuffer, handles CPU exceptions and hardware interrupts, sets up
paging, and takes keyboard input — all running inside QEMU.

This is a `#![no_std]` / `#![no_main]` freestanding kernel: it links against `core`/`alloc`
only (no operating system underneath it), and it is compiled for a **custom bare-metal
target** (`x86_64-my_os.json`) using an unstable `build-std` toolchain.

---

## What it currently does

When booted, the kernel:

1. Initializes the **GDT + TSS** (with a dedicated Interrupt Stack Table entry for the
   double-fault handler so stack overflows are handled cleanly).
2. Loads the **IDT** with handlers for `breakpoint`, `double fault`, `page fault`, the
   **timer** (IRQ0) and the **PS/2 keyboard** (IRQ1).
3. Remaps and unmasks the legacy **8259 PIC** and enables interrupts.
4. Clears the **UEFI framebuffer** and installs a text console (`print!` / `println!`)
   that renders an 8×8 bitmap font, supports scrolling, backspace, and a blinking cursor
   driven by the timer interrupt.
5. Sets up **paging**: reads the bootloader's physical-memory offset, builds an
   `OffsetPageTable`, and a `BootInfoFrameAllocator` that hands out usable physical frames
   from the bootloader's memory map. It then demonstrates a manual page mapping.
6. Types characters you press on the keyboard to the screen, then idles in an `hlt` loop.

There is also a **serial driver** (`serial_println!` over UART `0x3F8`) used mainly for
test output, and a small **custom test framework** that runs kernel tests inside QEMU
headless and reports pass/fail via QEMU's `isa-debug-exit` device.

---

## Repository layout

```
my_os/
├── Cargo.toml              # Cargo workspace (members: kernel, builder)
├── rust-toolchain.toml     # Pins the nightly toolchain + required components
├── x86_64-my_os.json       # Custom bare-metal target spec (used by the builder)
│
├── kernel/                 # The OS itself  (package name: `my_os`)
│   ├── .cargo/config.toml  # build-std flags + custom target + `cargo test` runner
│   ├── x86_64-my_os.json   # Same target spec, used when running `cargo test` here
│   └── src/
│       ├── main.rs         # Kernel entry point (kernel_main)
│       ├── lib.rs          # init(), test framework, QEMU exit, hlt_loop
│       ├── gdt.rs          # GDT + TSS
│       ├── interrupts.rs   # IDT, PIC, timer/keyboard/exception handlers
│       ├── framebuffer.rs  # Framebuffer text console + print!/println!
│       ├── memory.rs       # Paging + frame allocator
│       └── serial.rs       # UART serial driver + serial_println!
│   └── tests/              # Integration tests (basic_boot, breakpoint, ...)
│
├── builder/                # Host-side build+run orchestrator (a normal std binary)
│   └── src/
│       ├── main.rs         # Builds kernel → makes UEFI image → launches QEMU
│       └── test_runner.rs  # `cargo test` runner: boots a test binary in QEMU
│
└── firmware/x64/code.fd    # OVMF UEFI firmware for QEMU (checked in)
```

### How the build/run pipeline works

The kernel cannot just be `cargo run`, because it targets bare metal. The **`builder`**
crate is a normal host program that automates the whole thing (`builder/src/main.rs`):

1. Shells out to `cargo build` for the `my_os` package against `x86_64-my_os.json` with the
   `build-std` unstable flags.
2. Uses `bootloader::UefiBoot` to wrap the resulting kernel ELF into a bootable UEFI disk
   image at `target/uefi.img`.
3. Launches `qemu-system-x86_64` with the OVMF firmware (`firmware/x64/code.fd`) and that
   disk image.

So **running the OS = running the builder.**

---

## Prerequisites

### 1. Rust nightly (required)

The project needs nightly for `build-std`, the JSON target spec, `abi_x86_interrupt`,
custom test frameworks, etc. `rust-toolchain.toml` will auto-select it, but the toolchain
and its components must be installed first:

```powershell
rustup toolchain install nightly
rustup component add rust-src llvm-tools-preview --toolchain nightly
```

> The `rust-toolchain.toml` in this repo pins `channel = "nightly"`, so once nightly is
> installed every `cargo` command in this folder automatically uses it — you don't need
> `cargo +nightly`.

### 2. QEMU on your PATH

QEMU is required to actually boot the OS. On this machine it is installed at
`C:\Program Files\qemu\` but is **not on PATH**. Add it (one-time, per user):

```powershell
[Environment]::SetEnvironmentVariable(
    "Path",
    $env:Path + ";C:\Program Files\qemu",
    "User")
```

Then **open a new terminal** and verify:

```powershell
qemu-system-x86_64 --version
```

---

## Build & run

From the **workspace root** (`my_os/`):

```powershell
cargo run -p builder --bin builder
```

This builds the kernel, produces `target/uefi.img`, and opens a QEMU window running the OS.
You can type and characters appear on screen; the block cursor blinks.

> Note: `-p builder` alone is ambiguous because the builder package has two binaries
> (`builder` and `test-runner`), so `--bin builder` is required.

---

## Running the tests

The kernel's tests boot as real kernels inside QEMU (headless), print results over serial,
and signal success/failure through QEMU's `isa-debug-exit` device. This is wired up in
`kernel/.cargo/config.toml`, which only applies when cargo runs **inside the `kernel`
directory**:

```powershell
cd kernel
cargo test
```

A test run exits QEMU with code `33` on success (mapped to a passing test), driven by
`builder/src/test_runner.rs`.

---

## Troubleshooting

- **`error: target tuple in channel name ...`** — an old `rust-toolchain.toml` used
  `channel = "nightly-x86_64-pc-windows-gnu"`, which modern rustup rejects. It is now
  `channel = "nightly"`. If you see this again, that's the fix.
- **`qemu-system-x86_64: command not found` / `not recognized`** — QEMU isn't on PATH.
  See the prerequisites above and restart your terminal.
- **`error: "rust-src" not found` / build-std errors** — install the components:
  `rustup component add rust-src llvm-tools-preview --toolchain nightly`.
- **No QEMU window appears / build succeeds but nothing runs** — make sure you ran the
  `builder` binary (`--bin builder`), not just `cargo build`.

---

## Notes

- Panics print to both the framebuffer and serial, then halt.
- The `builder` crate lists `ovmf-prebuilt` / `ureq` as dependencies, but the current code
  uses the checked-in `firmware/x64/code.fd` directly, so no firmware download is needed.
- Target: UEFI x86-64, soft-float, red-zone disabled, `panic = "abort"`.
