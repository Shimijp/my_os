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

/// Invoke a breakpoint exception and verify execution continues.
/// This tests that the IDT breakpoint handler is correctly installed.
#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}
