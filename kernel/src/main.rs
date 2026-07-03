#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]
extern crate alloc;

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;
use bootloader_api::BootInfo;
use bootloader_api::config::Mapping;
use core::panic::PanicInfo;
use my_os::memory::{BootInfoFrameAllocator, init};
use my_os::{allocator, print, println, serial_println};
use x86_64::VirtAddr;
#[panic_handler]
#[cfg(not(test))]
fn panic(_info: &PanicInfo) -> ! {
    println!("{}", _info);
    serial_println!("{}", _info);
    loop {}
}

//mandatory for paging map
pub static BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

bootloader_api::entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

#[unsafe(no_mangle)]
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    my_os::init();

    /*I hate the fact that this in main and not some function, but the borrow checker fought me and i have lost(the will to live)
    so here it shall remain for now
     */
    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        let info = framebuffer.info();
        let buffer = framebuffer.buffer_mut();
        buffer.fill(0);
        my_os::init_framebuffer(buffer, info);
    } else {
        panic!("no framebuffer found!");
    }

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    let mut mapper = unsafe { init(phys_mem_offset) };

    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    // map an unused page

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    let heap_value = Box::new(41);
    println!("heap_value at {:p}", heap_value);

    // create a dynamically sized vector
    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at {:p}", vec.as_slice());
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!("current reference count is {}", Rc::strong_count(&cloned_reference));
    core::mem::drop(reference_counted);
    println!("reference count is {} now", Rc::strong_count(&cloned_reference));




    println!("It did not crash!");





    my_os::hlt_loop();
}
