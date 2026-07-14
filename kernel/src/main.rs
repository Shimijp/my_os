#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::{format, vec};
use alloc::vec::Vec;
use bootloader_api::BootInfo;
use bootloader_api::config::Mapping;
use core::panic::PanicInfo;

use x86_64::instructions::hlt;
use my_os::memory::{BootInfoFrameAllocator, init, FRAME_ALLOCATOR, PHYS_MEM_OFFSET};
use my_os::{allocator,  println, serial_println};
use x86_64::VirtAddr;
use my_os::scheduler::{get_current_task_id, HAS_TERMINATED_TASKS, SCHEDULER};
use my_os::task::Task;

#[panic_handler]
#[cfg(not(test))]
fn panic(_info: &PanicInfo) -> ! {
    println!("{}", _info);
    serial_println!("{}", _info);
    loop {}
}
use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};
use my_os::mutex::Mutex;

pub fn init_fpu() {
    unsafe {

        let mut cr0 = Cr0::read();
        cr0.remove(Cr0Flags::EMULATE_COPROCESSOR);
        cr0.insert(Cr0Flags::MONITOR_COPROCESSOR);
        Cr0::write(cr0);

        let mut cr4 = Cr4::read();
        cr4.insert(Cr4Flags::OSFXSR);
        cr4.insert(Cr4Flags::OSXMMEXCPT_ENABLE);
        Cr4::write(cr4);
    }
}

//mandatory for paging map
pub static BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::FixedAddress(0xFFFF_8000_0000_0000));
    config
};

bootloader_api::entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);
fn increase() -> u64
{
    let mut lock = MY_MUTEX.lock();
    *lock += 1;
    println!("hello, i am running my pid is {}, my mutex value is {}", get_current_task_id(), *lock);
    0
}
static MY_MUTEX: Mutex<u8> = Mutex::new(0);

fn task_5() -> u64
{

    let vec = (2..10_000).filter(|&n| is_prime(n)).collect::<Vec<u64>>();
    println!("task 5 found {} primes", vec.len());


    0
}
fn is_prime(n: u64) -> bool {
    if n <= 1 {
        return false;
    }
    let mut  j = 2;
    while j * j <= n {
        if n % j == 0 {
            return false;
        }
        j += 1;
    }
    true
}

//stress test ram and cpu by calculating primes up to 1 million and printing them


#[unsafe(no_mangle)]
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    my_os::init();
    /*I hate the fact that this in main and not some function, but the borrow checker fought me and I have lost(the will to live)
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
    println!("physical memory offset is {:?}", phys_mem_offset);
    let mut mapper = unsafe { init(phys_mem_offset) };
    let  frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    // map an unused page
    let mut glob_frame_alloc = FRAME_ALLOCATOR.lock();
    *glob_frame_alloc = Some(frame_allocator);

    let mut glob_offset = PHYS_MEM_OFFSET.lock();
    *glob_offset = phys_mem_offset;



    allocator::init_heap(&mut mapper, glob_frame_alloc.as_mut().unwrap())
        .expect("heap initialization failed");

    drop(glob_frame_alloc);
    drop(glob_offset);
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
    drop(reference_counted);
    println!("reference count is {} now", Rc::strong_count(&cloned_reference));
    init_fpu();
    let scheduler = &SCHEDULER;
    for i in 0..7 {
        let task_name = format!("task_{}", i);
        let task_entry = match i {
            5 => task_5,
            _ => increase,
        };
        let task = Task::new(&task_name, task_entry);
        scheduler.add_task(task);
    }


    println!("It did not crash!");


    loop {

            if HAS_TERMINATED_TASKS.load(core::sync::atomic::Ordering::SeqCst)
            {
                SCHEDULER.clear_terminated_tasks();
            }

        hlt();
    }
}
