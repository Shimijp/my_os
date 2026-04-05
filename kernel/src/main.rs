#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]

use bootloader_api::BootInfo;
use bootloader_api::config::Mapping;
use core::panic::PanicInfo;
use my_os::memory::{BootInfoFrameAllocator, create_example_mapping, init};
use my_os::{println, serial_println};
use x86_64::VirtAddr;
use x86_64::structures::paging::Page;
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
    let page = Page::containing_address(VirtAddr::new(0));
    create_example_mapping(page, &mut mapper, &mut frame_allocator);

    // write the string `New!` to the screen through the new mapping
    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e) };
    let val = unsafe { page_ptr.offset(400).read_volatile() };
    println!("read {}", val);
    println!("it didnt panic! a true miracle!");

    my_os::hlt_loop();
}
