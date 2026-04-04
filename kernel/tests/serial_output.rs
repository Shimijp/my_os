#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(my_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use my_os::{serial_print, serial_println};

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
fn test_serial_print_no_newline() {
    serial_print!("no newline");
    serial_print!(" still going");
    serial_println!(); // flush line
}

#[test_case]
fn test_serial_println_empty() {
    serial_println!();
}

#[test_case]
fn test_serial_println_numbers() {
    serial_println!("u8={} u16={} u32={} u64={}", 1u8, 2u16, 3u32, 4u64);
}

#[test_case]
fn test_serial_println_long_string() {
    let long = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    serial_println!("{}", long);
}

#[test_case]
fn test_serial_println_special_chars() {
    serial_println!("tabs\there");
    serial_println!("backslash\\end");
    serial_println!("braces {{}}");
}

#[test_case]
fn test_serial_println_many() {
    for i in 0..100 {
        serial_println!("serial line {}", i);
    }
}

#[test_case]
fn test_serial_print_mixed() {
    serial_print!("a");
    serial_print!("b");
    serial_print!("c");
    serial_println!("d");
}
