#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]

use core::ops::Deref;
use core::panic::PanicInfo;
use bootloader_api::BootInfo;
use bootloader_api::config::Mapping;
use x86_64::structures::paging::PageTable;
use x86_64::VirtAddr;
use my_os::{gdt, memory, println, serial_println};

#[panic_handler]
#[cfg(not(test))]
fn panic(_info: & PanicInfo) ->!
{
    println!("{}", _info);
    serial_println!("{}", _info);

    loop{}
}
//mandatory for paging map
pub static BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

bootloader_api::entry_point!(kernel_main, config =  &BOOTLOADER_CONFIG);


#[unsafe(no_mangle)]
fn kernel_main(boot_info: &'static mut BootInfo) -> !
{
    use my_os::memory::active_label_4_table;

    my_os::init();

    /*I hate the fact that this  in main and not some function, but the borrow checker fought me and i have lost(the will to live)
    so here it shall remain for now
     */
    if let Some(framebuffer) = boot_info.framebuffer.as_mut()
    {
        let info = framebuffer.info();
        let buffer = framebuffer.buffer_mut();
        buffer.fill(0);
        my_os::init_framebuffer(buffer, info);
    } else {
        panic!("no framebuffer found!");
    }


    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    let l4_table = unsafe { active_label_4_table(phys_mem_offset)};
    let addresses = [
        // the identity-mapped vga buffer page
        0xb8000,
        // some code page
        0x201008,
        // some stack page
        0x0100_0020_1a10,
        phys_mem_offset.as_u64(),
        // virtual address mapped to physical address 0
    ];
    for &address in &addresses
    {
        let virt = VirtAddr::new(address);
        let phys = unsafe {memory::translate_addr(virt, phys_mem_offset)};
        println!("{:?} -> {:?}", virt, phys);

    }








    use x86_64::registers::control::Cr3;

    let (level_4_page_table, _) = Cr3::read();
    println!("Level 4 page table at: {:?}", level_4_page_table.start_address());


    my_os::hlt_loop();
}
