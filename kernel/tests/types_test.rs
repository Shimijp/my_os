#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(my_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use my_os::QemuExitCode;
use my_os::interrupts::{InterruptIndex, PIC_1_OFFSET, PIC_2_OFFSET};
use my_os::gdt::DOUBLE_FAULT_IST_INDEX;
use my_os::framebuffer::Rgba;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    my_os::test_panic_handler(info)
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

// --- QemuExitCode tests ---

#[test_case]
fn test_qemu_exit_code_success_value() {
    assert_eq!(QemuExitCode::Success as u32, 0x10);
}

#[test_case]
fn test_qemu_exit_code_failed_value() {
    assert_eq!(QemuExitCode::Failed as u32, 0x11);
}

#[test_case]
fn test_qemu_exit_code_not_equal() {
    assert_ne!(QemuExitCode::Success, QemuExitCode::Failed);
}

#[test_case]
fn test_qemu_exit_code_eq() {
    assert_eq!(QemuExitCode::Success, QemuExitCode::Success);
    assert_eq!(QemuExitCode::Failed, QemuExitCode::Failed);
}

#[test_case]
fn test_qemu_exit_code_copy() {
    let a = QemuExitCode::Success;
    let b = a;
    let c = a; // still usable because Copy
    assert_eq!(b, c);
}

#[test_case]
fn test_qemu_exit_code_clone() {
    let a = QemuExitCode::Success;
    let b = a.clone();
    assert_eq!(a, b);
}

// --- InterruptIndex tests ---

#[test_case]
fn test_interrupt_index_timer_value() {
    assert_eq!(InterruptIndex::Timer.as_u8(), PIC_1_OFFSET);
}

#[test_case]
fn test_interrupt_index_keyboard_value() {
    assert_eq!(InterruptIndex::Keyboard.as_u8(), PIC_1_OFFSET + 1);
}

#[test_case]
fn test_interrupt_index_timer_as_usize() {
    assert_eq!(InterruptIndex::Timer.as_usize(), PIC_1_OFFSET as usize);
}

#[test_case]
fn test_interrupt_index_keyboard_as_usize() {
    assert_eq!(InterruptIndex::Keyboard.as_usize(), PIC_1_OFFSET as usize + 1);
}

#[test_case]
fn test_interrupt_index_as_u8_as_usize_consistent() {
    let timer_u8 = InterruptIndex::Timer.as_u8();
    let timer_usize = InterruptIndex::Timer.as_usize();
    assert_eq!(timer_u8 as usize, timer_usize);
}

// --- PIC offset tests ---

#[test_case]
fn test_pic_1_offset() {
    assert_eq!(PIC_1_OFFSET, 32);
}

#[test_case]
fn test_pic_2_offset() {
    assert_eq!(PIC_2_OFFSET, 40);
}

#[test_case]
fn test_pic_offsets_spacing() {
    assert_eq!(PIC_2_OFFSET - PIC_1_OFFSET, 8);
}

// --- GDT constants ---

#[test_case]
fn test_double_fault_ist_index() {
    assert_eq!(DOUBLE_FAULT_IST_INDEX, 0);
}

// --- Rgba tests ---

#[test_case]
fn test_rgba_new() {
    let _rgba = Rgba::new(0xFF00FF00);
}

#[test_case]
fn test_rgba_new_zero() {
    let _rgba = Rgba::new(0x00000000);
}

#[test_case]
fn test_rgba_new_max() {
    let _rgba = Rgba::new(0xFFFFFFFF);
}

#[test_case]
fn test_rgba_copy() {
    let a = Rgba::new(0xAABBCCDD);
    let b = a;
    let _c = a; // still valid because Copy
    let _d = b;
}

#[test_case]
fn test_rgba_clone() {
    let a = Rgba::new(0x12345678);
    let _b = a.clone();
}
