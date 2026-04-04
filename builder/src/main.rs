use std::path::PathBuf;
use std::process::Command;

fn main() {
    // === שלב אפס: הגדרת נתיבים אבסולוטית ===
    // העוגן שלנו: התיקייה של ה-builder (my_os/builder)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    // תיקיית השורש של הפרויקט: אנחנו עולים רמה אחת למעלה ל-my_os
    let root_dir = PathBuf::from(manifest_dir)
        .join("..")
        .canonicalize()
        .expect("FATAL: Failed to find project root directory");

    // נגזרות הנתיבים מתוך השורש (הכל אבסולוטי עכשיו!)
    let kernel_path = root_dir.join("target/x86_64-my_os/debug/my_os");
    let uefi_img_path = root_dir.join("target/uefi.img");
    let firmware_dir = root_dir.join("firmware/x64");


    // === Step 1: Build the kernel ===
    println!("Step 1: Building the kernel...");

    // שים לב: אנחנו מוסיפים current_dir כדי לוודא ש-cargo רץ מהשורש של הפרויקט
    let build_status = Command::new("cargo")
        .current_dir(&root_dir)
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


    // === Step 2: Create the UEFI disk image ===
    println!("Step 2: Creating UEFI disk image...");

    bootloader::UefiBoot::new(&kernel_path)
        .create_disk_image(&uefi_img_path)
        .expect("Failed to create UEFI disk image");


    // === Step 3: Run QEMU ===
    println!("Step 3: Running QEMU...");

    let ovmf_firmware = firmware_dir.join("code.fd")
        .to_string_lossy().replace('\\', "/");

    let uefi_qemu_path = uefi_img_path
        .to_string_lossy().replace('\\', "/");



    let mut qemu_cmd = Command::new("qemu-system-x86_64");
    qemu_cmd.arg("-drive").arg(format!("if=pflash,format=raw,readonly=on,file={}", ovmf_firmware));
    qemu_cmd.arg("-drive").arg(format!("format=raw,file={}", uefi_qemu_path));

    let mut child = qemu_cmd.spawn().expect("Failed to start QEMU");
    child.wait().expect("QEMU crashed or failed to wait");
}