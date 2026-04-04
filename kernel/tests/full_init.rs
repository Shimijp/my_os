#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(my_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    my_os::test_panic_handler(info)
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    my_os::init();
    test_main();
    loop {}
}

/// Test that the full init sequence (GDT + IDT + PIC + interrupts) completes.
#[test_case]
fn test_init_completes() {
    // If we reach here, init() in _start succeeded
}

/// After init, hardware interrupts should be enabled.
#[test_case]
fn test_interrupts_enabled() {
    assert!(x86_64::instructions::interrupts::are_enabled());
}

/// After init, hlt should return because the timer interrupt wakes the CPU.
#[test_case]
fn test_hlt_resumes() {
    x86_64::instructions::hlt();
    // reaching here means an interrupt woke us up
}

/// Test that a breakpoint exception after full init is handled correctly.
#[test_case]
fn test_breakpoint_after_full_init() {
    x86_64::instructions::interrupts::int3();
}

/// Test that multiple hlt cycles work (timer keeps firing).
#[test_case]
fn test_multiple_hlt_cycles() {
    for _ in 0..5 {
        x86_64::instructions::hlt();
    }
}
