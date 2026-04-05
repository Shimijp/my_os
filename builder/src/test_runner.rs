use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let kernel_binary = std::env::args().nth(1).expect(
        "Usage: test-runner <kernel-binary-path>"
    );
    let kernel_path = PathBuf::from(&kernel_binary);

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let root_dir = PathBuf::from(manifest_dir)
        .join("..")
        .canonicalize()
        .expect("Failed to find project root directory");

    let uefi_img_path = root_dir.join("target/test-uefi.img");
    let firmware_dir = root_dir.join("firmware/x64");

    // Create UEFI disk image from the test binary
    bootloader::UefiBoot::new(&kernel_path)
        .create_disk_image(&uefi_img_path)
        .expect("Failed to create UEFI disk image for test");

    let ovmf_firmware = firmware_dir.join("code.fd")
        .to_string_lossy().replace('\\', "/");
    let uefi_qemu_path = uefi_img_path
        .to_string_lossy().replace('\\', "/");

    // Run QEMU with test flags: debug-exit device, serial to stdio, no display
    let status = Command::new("qemu-system-x86_64")
        .arg("-drive").arg(format!("if=pflash,format=raw,readonly=on,file={}", ovmf_firmware))
        .arg("-drive").arg(format!("format=raw,file={}", uefi_qemu_path))
        .arg("-device").arg("isa-debug-exit,iobase=0xf4,iosize=0x04")
        .arg("-serial").arg("stdio")
        .arg("-display").arg("none")
        .status()
        .expect("Failed to start QEMU");

    let qemu_exit = status.code().unwrap_or(1);
    // QEMU exit code: (value_written_to_port << 1) | 1
    // Success = 0x10 written -> (0x10 << 1) | 1 = 33
    if qemu_exit == 33 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}