#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(my_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use bootloader_api::BootInfo;
use bootloader_api::config::Mapping;
use core::panic::PanicInfo;
use my_os::allocator;
use my_os::memory::BootInfoFrameAllocator;
use x86_64::VirtAddr;
use x86_64::instructions::interrupts::without_interrupts;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    my_os::test_panic_handler(info)
}

pub static BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

bootloader_api::entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    my_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    let mut mapper = unsafe { my_os::memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    // Force the scheduler's lazy init (which allocates) to happen here, with
    // interrupts off, instead of for the first time inside the timer handler.
    without_interrupts(|| {
        let _ = my_os::scheduler::SCHEDULER.inner.lock();
    });

    test_main();
    my_os::hlt_loop();
}

#[test_case]
fn simple_box_allocation() {
    let v1 = Box::new(41);
    let v2 = Box::new(13);
    assert_eq!(*v1, 41);
    assert_eq!(*v2, 13);
}

#[test_case]
fn large_vec_sum() {
    let n = 1000u64;
    let mut v = Vec::new();
    for i in 0..n {
        v.push(i);
    }
    assert_eq!(v.iter().sum::<u64>(), (n - 1) * n / 2);
}

#[test_case]
fn vec_growth_reallocates_correctly() {
    let mut v = Vec::new();
    for i in 0..10_000usize {
        v.push(i);
    }
    assert_eq!(v.len(), 10_000);
    assert_eq!(v[0], 0);
    assert_eq!(v[9_999], 9_999);
}

/// Churns through 32 MB of allocations on a 16 MB heap. This only passes if
/// dealloc really returns blocks to the free list and alloc reuses them.
#[test_case]
fn many_boxes_reuse_freed_memory() {
    const BLOCK: usize = 1024;
    const ITERATIONS: usize = 32 * 1024;
    for _ in 0..ITERATIONS {
        let x = Box::new([0u8; BLOCK]);
        assert_eq!(x[0], 0);
        assert_eq!(x[BLOCK - 1], 0);
    }
}

/// A long-lived allocation must survive heavy alloc/free churn around it
/// (catches free-list corruption that overwrites live blocks).
#[test_case]
fn long_lived_allocation_survives_churn() {
    let long_lived = Box::new(1u64);
    for i in 0..10_000u64 {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
    assert_eq!(*long_lived, 1);
}

/// Free a middle block and allocate again — exercises first-fit reuse and
/// coalescing with non-trivial neighbors.
#[test_case]
fn interleaved_alloc_free() {
    let a = Box::new([1u8; 128]);
    let b = Box::new([2u8; 256]);
    let c = Box::new([3u8; 512]);
    drop(b);
    let d = Box::new([4u8; 64]);
    assert_eq!(a[127], 1);
    assert_eq!(c[511], 3);
    assert_eq!(d[63], 4);
}

#[test_case]
fn rc_reference_counting() {
    let rc = Rc::new(vec![1, 2, 3]);
    let clone = rc.clone();
    assert_eq!(Rc::strong_count(&clone), 2);
    drop(rc);
    assert_eq!(Rc::strong_count(&clone), 1);
    assert_eq!(clone.iter().sum::<i32>(), 6);
}

#[test_case]
fn string_formatting_allocates() {
    let s = format!("task_{}", 7);
    assert_eq!(s, "task_7");
    let mut owned = String::new();
    for _ in 0..100 {
        owned.push_str("ab");
    }
    assert_eq!(owned.len(), 200);
}

/// The allocator claims to honor arbitrary alignment via padding — ask for a
/// page-aligned block and make sure the whole block is usable.
#[test_case]
fn page_aligned_raw_allocation() {
    use core::alloc::Layout;
    let layout = Layout::from_size_align(4096, 4096).unwrap();
    unsafe {
        let ptr = alloc::alloc::alloc(layout);
        assert!(!ptr.is_null(), "allocator returned null for aligned alloc");
        assert_eq!(ptr as usize % 4096, 0, "pointer is not 4096-aligned");
        ptr.write_bytes(0xAB, 4096);
        assert_eq!(*ptr, 0xAB);
        assert_eq!(*ptr.add(4095), 0xAB);
        alloc::alloc::dealloc(ptr, layout);
    }
}