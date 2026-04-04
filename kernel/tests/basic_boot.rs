#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(my_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use my_os::{println, serial_println};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    my_os::test_panic_handler(info)
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for i in 0..200 {
        println!("test_println_many line {}", i);
    }
}

#[test_case]
fn test_println_format_args() {
    let x = 42;
    println!("formatted: {} + {} = {}", x, x, x + x);
}

#[test_case]
fn test_println_empty() {
    println!();
}

#[test_case]
fn test_serial_println_simple() {
    serial_println!("test_serial_println_simple output");
}

#[test_case]
fn test_serial_println_format() {
    serial_println!("serial format: {} {}", "hello", 42);
}