#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(my_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

//! Tests for preemptive multitasking: task creation, timer-driven context
//! switching, round-robin fairness, yielding, and allocation inside tasks.
//!
//! Every test communicates with its task(s) through atomics and waits with a
//! bounded hlt loop — the QEMU test runner has no timeout, so a test must
//! panic on its own rather than hang.

extern crate alloc;

use alloc::vec::Vec;
use bootloader_api::BootInfo;
use bootloader_api::config::Mapping;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use my_os::allocator;
use my_os::memory::BootInfoFrameAllocator;
use my_os::scheduler::{SCHEDULER, yield_now};
use my_os::task::Task;
use x86_64::VirtAddr;
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};

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

// Same FPU/SSE setup as kernel_main in main.rs does before spawning tasks.
fn init_fpu() {
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

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    my_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    let mut mapper = unsafe { my_os::memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    // Force the scheduler's lazy init (which allocates) to happen here, with
    // interrupts off, instead of for the first time inside the timer handler.
    without_interrupts(|| {
        let _ = SCHEDULER.inner.lock();
    });
    init_fpu();

    test_main();
    my_os::hlt_loop();
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Create a task, remember its id, and hand it to the scheduler.
fn spawn(name: &str, entry: fn() -> u64) -> usize {
    let task = Task::new(name, entry);
    let id = task.id;
    SCHEDULER.add_task(task);
    id
}

/// Wait (hlt per timer tick) until `cond` holds, up to `max_ticks` of this
/// task's own timeslices. Returns whether the condition was ever met.
fn wait_until(max_ticks: usize, cond: impl Fn() -> bool) -> bool {
    for _ in 0..max_ticks {
        if cond() {
            return true;
        }
        x86_64::instructions::hlt();
    }
    cond()
}

/// Look a task up by id in the scheduler. `None` = no such task,
/// `Some(exit_code)` = the task's current exit code field.
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

const MAX_WAIT: usize = 500;

// ---------------------------------------------------------------------------
// 1. a spawned task actually runs (timer preempts the boot task)
// ---------------------------------------------------------------------------

static SINGLE_TASK_RAN: AtomicUsize = AtomicUsize::new(0);

fn single_task() -> u64 {
    SINGLE_TASK_RAN.store(42, Ordering::SeqCst);
    0
}

#[test_case]
fn test_task_runs_and_completes() {
    spawn("single", single_task);
    let ran = wait_until(MAX_WAIT, || SINGLE_TASK_RAN.load(Ordering::SeqCst) == 42);
    assert!(ran, "spawned task was never scheduled");
}

// ---------------------------------------------------------------------------
// 2. a task's return value reaches sys_exit and is recorded as its exit code
// ---------------------------------------------------------------------------

fn exit_code_task() -> u64 {
    7
}

#[test_case]
fn test_task_exit_code_recorded() {
    let id = spawn("exit_code", exit_code_task);
    let recorded = wait_until(MAX_WAIT, || find_task_exit_code(id) == Some(Some(7)));
    assert!(recorded, "task exit code 7 was never recorded by sys_exit");
}

// ---------------------------------------------------------------------------
// 3. many tasks all get scheduled
// ---------------------------------------------------------------------------

static RUN_COUNT: AtomicUsize = AtomicUsize::new(0);

fn counting_task() -> u64 {
    RUN_COUNT.fetch_add(1, Ordering::SeqCst);
    0
}

#[test_case]
fn test_multiple_tasks_all_run() {
    for _ in 0..5 {
        spawn("counting", counting_task);
    }
    let all_ran = wait_until(MAX_WAIT, || RUN_COUNT.load(Ordering::SeqCst) >= 5);
    assert!(all_ran, "only {} of 5 tasks ran", RUN_COUNT.load(Ordering::SeqCst));
}

// ---------------------------------------------------------------------------
// 4. heap allocation works from inside a task (allocator + scheduler combined)
// ---------------------------------------------------------------------------

static ALLOC_TASK_SUM: AtomicU64 = AtomicU64::new(0);
static ALLOC_TASK_DONE: AtomicBool = AtomicBool::new(false);

fn allocating_task() -> u64 {
    let mut v = Vec::new();
    for i in 0..1000u64 {
        v.push(i);
    }
    ALLOC_TASK_SUM.store(v.iter().sum(), Ordering::SeqCst);
    ALLOC_TASK_DONE.store(true, Ordering::SeqCst);
    0
}

#[test_case]
fn test_alloc_inside_task() {
    spawn("allocating", allocating_task);
    let done = wait_until(MAX_WAIT, || ALLOC_TASK_DONE.load(Ordering::SeqCst));
    assert!(done, "allocating task never finished");
    assert_eq!(ALLOC_TASK_SUM.load(Ordering::SeqCst), 999 * 1000 / 2);
}

// ---------------------------------------------------------------------------
// 5. yield_now (int 0x81) hands the CPU to other tasks without waiting for
//    the timer
// ---------------------------------------------------------------------------

static YIELD_FLAG: AtomicBool = AtomicBool::new(false);

fn yield_flag_task() -> u64 {
    YIELD_FLAG.store(true, Ordering::SeqCst);
    0
}

#[test_case]
fn test_yield_now_schedules_other_tasks() {
    spawn("yield_flag", yield_flag_task);
    let mut seen = false;
    for _ in 0..100 {
        if YIELD_FLAG.load(Ordering::SeqCst) {
            seen = true;
            break;
        }
        yield_now();
    }
    assert!(seen, "task never ran despite repeated yields");
}

// ---------------------------------------------------------------------------
// 6. round robin is preemptive: two busy-looping tasks that never yield or
//    hlt both make progress at the same time, while the boot task (this test)
//    also keeps running
// ---------------------------------------------------------------------------

static SPIN_A: AtomicUsize = AtomicUsize::new(0);
static SPIN_B: AtomicUsize = AtomicUsize::new(0);
static STOP_SPINNING: AtomicBool = AtomicBool::new(false);
static SPIN_A_DONE: AtomicBool = AtomicBool::new(false);
static SPIN_B_DONE: AtomicBool = AtomicBool::new(false);

fn spin_task_a() -> u64 {
    while !STOP_SPINNING.load(Ordering::SeqCst) {
        SPIN_A.fetch_add(1, Ordering::Relaxed);
    }
    SPIN_A_DONE.store(true, Ordering::SeqCst);
    0
}

fn spin_task_b() -> u64 {
    while !STOP_SPINNING.load(Ordering::SeqCst) {
        SPIN_B.fetch_add(1, Ordering::Relaxed);
    }
    SPIN_B_DONE.store(true, Ordering::SeqCst);
    0
}

#[test_case]
fn test_preemptive_interleaving() {
    spawn("spin_a", spin_task_a);
    spawn("spin_b", spin_task_b);

    // Both spinners progressing while neither exits proves the timer is
    // forcibly switching between them (and back to us to observe it).
    let both_progressed = wait_until(MAX_WAIT, || {
        SPIN_A.load(Ordering::Relaxed) > 10_000 && SPIN_B.load(Ordering::Relaxed) > 10_000
    });
    assert!(
        both_progressed,
        "no interleaving: spin_a={} spin_b={}",
        SPIN_A.load(Ordering::Relaxed),
        SPIN_B.load(Ordering::Relaxed)
    );

    STOP_SPINNING.store(true, Ordering::SeqCst);
    let both_exited = wait_until(MAX_WAIT, || {
        SPIN_A_DONE.load(Ordering::SeqCst) && SPIN_B_DONE.load(Ordering::SeqCst)
    });
    assert!(both_exited, "spinner tasks did not stop after the stop flag");
}

// ---------------------------------------------------------------------------
// 7. the boot task's stack survives being switched away from and back many
//    times (catches save/restore bugs in switch.s)
// ---------------------------------------------------------------------------

fn stack_user_task() -> u64 {
    // Do some stack-heavy work on the task's own stack.
    let mut local = [0u64; 64];
    for (i, slot) in local.iter_mut().enumerate() {
        *slot = i as u64 * 3;
    }
    local.iter().sum::<u64>()
}

#[test_case]
fn test_boot_stack_intact_across_switches() {
    let mut stack_data = [0u64; 32];
    for (i, slot) in stack_data.iter_mut().enumerate() {
        *slot = 0x1234_5678_9ABC_DEF0 ^ (i as u64);
    }

    spawn("stack_user", stack_user_task);
    // Get preempted a bunch of times while the other task uses its stack.
    for _ in 0..20 {
        x86_64::instructions::hlt();
    }

    for (i, slot) in stack_data.iter().enumerate() {
        assert_eq!(
            *slot,
            0x1234_5678_9ABC_DEF0 ^ (i as u64),
            "boot stack corrupted at slot {}",
            i
        );
    }
}