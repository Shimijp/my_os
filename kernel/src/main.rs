#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use bootloader_api::BootInfo;
use my_os::{ println, serial_println};

#[panic_handler]
#[cfg(not(test))]
fn panic(_info: & PanicInfo) ->!
{
    println!("{}", _info);
    serial_println!("{}", _info);

    loop{}
}



bootloader_api::entry_point!(kernel_main);


#[unsafe(no_mangle)]
fn kernel_main(boot_info: &'static mut BootInfo) ->!
{

    my_os::init();
    if let Some(framebuffer) = boot_info.framebuffer.as_mut()
    {
        let info = framebuffer.info();
        let  buffer = framebuffer.buffer_mut();
        buffer.fill(0);
        my_os::init_framebuffer(buffer, info);
    }

    else {
        panic!("no framebuffer found!");
    }






    println!("hello");


    my_os::hlt_stop();
}
