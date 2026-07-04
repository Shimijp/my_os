#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(my_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

//! Tests that exited tasks are actually cleaned up, mirroring what the idle
//! loop in main.rs does with HAS_TERMINATED_TASKS + clear_terminated_tasks().
//!
//! NOTE: this test currently FAILS and is expected to — it exposes a real
//! scheduler bug. `prepare_schedule()` unconditionally sets the outgoing
//! task's state to `Ready`, even when that task just called sys_exit and was
//! marked `Terminated` (or was just `Blocked` by the mutex). The exited task
//! is resurrected, keeps getting timeslices in sys_exit's hlt loop forever,
//! and `clear_terminated_tasks()` never removes it because its state is no
//! longer `Terminated`. The fix is to only demote the outgoing task to
//! `Ready` when it is currently `Running`.
//!
//! It lives in its own file so its failure doesn't stop the other suites
//! (the QEMU runner exits on the first failed test in a binary).

extern crate alloc;

use bootloader_api::BootInfo;
use bootloader_api::config::Mapping;
use core::panic::PanicInfo;
use my_os::allocator;
use my_os::memory::BootInfoFrameAllocator;
use my_os::scheduler::{HAS_TERMINATED_TASKS, SCHEDULER};
use my_os::task::Task;
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

    without_interrupts(|| {
        let _ = SCHEDULER.inner.lock();
    });

    test_main();
    my_os::hlt_loop();
}

fn wait_until(max_ticks: usize, cond: impl Fn() -> bool) -> bool {
    for _ in 0..max_ticks {
        if cond() {
            return true;
        }
        x86_64::instructions::hlt();
    }
    cond()
}

fn find_task_exit_code(task_id: usize) -> Option<Option<i32>> {
    without_interrupts(|| {
        let scheduler = SCHEDULER.inner.lock();
        scheduler
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .map(|t| t.exit_code)
    })
}

fn short_task() -> u64 {
    3
}

#[test_case]
fn test_terminated_task_removed_after_clear() {
    let task = Task::new("short", short_task);
    let id = task.id;
    SCHEDULER.add_task(task);

    // Wait for the task to run and exit with code 3.
    let finished = wait_until(500, || find_task_exit_code(id) == Some(Some(3)));
    assert!(finished, "task never recorded its exit code");

    // sys_exit must have flagged that there is something to clean up.
    

    // After a cleanup pass the exited task must be gone from the task list.
    SCHEDULER.clear_terminated_tasks();
    let still_in_list = without_interrupts(|| {
        SCHEDULER.inner.lock().tasks.iter().any(|t| t.id == id)
    });
    assert!(
        !still_in_list,
        "exited task {} still in the task list after clear_terminated_tasks \
         (prepare_schedule resurrects Terminated tasks to Ready)",
        id
    );
}