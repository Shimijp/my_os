use std::path::PathBuf;
use std::process::Command;
use ovmf_prebuilt::{Arch, FileType, Prebuilt, Source};

fn main() {
    // Step 1: Build the kernel
    println!("Step 1: Building the kernel...");
    let build_status = Command::new("cargo")
        .arg("build")
        .arg("-Z").arg("json-target-spec")
        .arg("-Z").arg("build-std=core,compiler_builtins,alloc")
        .arg("-Z").arg("build-std-features=compiler-builtins-mem")
        .arg("--package").arg("my_os")
        .arg("--target").arg("x86_64-my_os.json")
        .status()
        .expect("Failed to run cargo build");

    if !build_status.success() {
        panic!("Kernel build failed! Check the errors above.");
    }

    // Step 2: Create the UEFI disk image
    println!("Step 2: Creating UEFI disk image...");

    let mut kernel_path = PathBuf::from("target");
    kernel_path.push("x86_64-my_os");
    kernel_path.push("debug");
    kernel_path.push("my_os");

    let mut uefi_path = PathBuf::from("target");
    uefi_path.push("uefi.img");

    bootloader::UefiBoot::new(&kernel_path)
        .create_disk_image(&uefi_path)
        .expect("Failed to create UEFI disk image");

    // Step 3: Run QEMU
    println!("Step 3: Running QEMU...");

    let prebuilt = Prebuilt::fetch(Source::LATEST, "target/ovmf")
        .expect("Failed to download OVMF firmware");

    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let ovmf_firmware = cwd.join(prebuilt.get_file(Arch::X64, FileType::Code))
        .to_string_lossy().replace('\\', "/");
    let uefi_path = cwd.join(&uefi_path)
        .to_string_lossy().replace('\\', "/");

    let mut qemu_cmd = Command::new("qemu-system-x86_64");
    qemu_cmd.arg("-drive").arg(format!("if=pflash,format=raw,readonly=on,file={}", ovmf_firmware));
    qemu_cmd.arg("-drive").arg(format!("format=raw,file={}", uefi_path));

    let mut child = qemu_cmd.spawn().expect("Failed to start QEMU");
    child.wait().expect("QEMU crashed or failed to wait");
}