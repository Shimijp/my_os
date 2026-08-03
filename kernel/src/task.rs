use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use x86_64::structures::paging::PhysFrame;
use crate::pml4::{ create_new_pml4};
use crate::println;
use crate::syscall::task_trampoline;

pub const STACK_SIZE: usize = 4096 * 4; // 16KB stack size
static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(0);
pub enum TaskState {
    New,
    Ready,
    Running,
    Waiting,
    Suspended,
    Zombie,
    Terminated,
    Blocked,
}


impl PartialEq for TaskState {
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other),
            (TaskState::New, TaskState::New) |
            (TaskState::Ready, TaskState::Ready) |
            (TaskState::Running, TaskState::Running) |
            (TaskState::Waiting, TaskState::Waiting) |
            (TaskState::Suspended, TaskState::Suspended) |
            (TaskState::Zombie, TaskState::Zombie) |
            (TaskState::Terminated, TaskState::Terminated) |
            (TaskState::Blocked, TaskState::Blocked)
        )
    }
}
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TaskContext {
    // Callee-saved registers
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,

    // CPU Flags
    pub rflags: u64,
    // Instruction Pointer (return address)
    pub rip: u64,
}


pub struct Task {
    pub id: usize,
    pub name: String,
    pub stack_pointer: usize,
    pub base_stack :  Option<usize>,
    pub state: TaskState,
    pub page_table: PhysFrame,
    pub priority: u8,
    pub start_time: u64,
    pub cpu_time: u64,
    pub memory_usage: usize,
    pub parent_id: Option<usize>,
    pub children: Vec<usize>,
    pub exit_code: Option<i32>,


}

impl Drop for Task {
    fn drop(&mut self) {
        if let Some(base) = self.base_stack {
            let layout = core::alloc::Layout::from_size_align(STACK_SIZE, 16).unwrap();
            unsafe { alloc::alloc::dealloc(base as *mut u8, layout) };
        }

    }
}
impl Task {
    pub fn new(name: &str, entry_point : fn() -> u64) -> Self {
        let id = NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst);
        let layout = core::alloc::Layout::from_size_align(STACK_SIZE, 16).unwrap();
        let base = unsafe { alloc::alloc::alloc(layout) } as usize;
        let stack_ptr_end=  base + STACK_SIZE;

        // Set up the stack for the new task
        let trampoline_ptr = (stack_ptr_end - size_of::<usize>()  ) as *mut usize;
        let stack_ptr = stack_ptr_end - size_of::<TaskContext>() - size_of::<usize>();
        let context = TaskContext {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            rflags: 0x202, // Default RFLAGS value
            rip: entry_point as u64,        // Set to the entry point of the task
        };
        unsafe {

            *trampoline_ptr = task_trampoline as usize;
            let context_ptr = stack_ptr as *mut TaskContext;
            *context_ptr = context;
        }
        let mut global_frame_allocator_guard = crate::memory::FRAME_ALLOCATOR.lock();
        let global_frame_allocator = global_frame_allocator_guard.as_mut()
            .expect("Frame allocator not initialized");
        Task {
            id,
            name: name.into(),
            stack_pointer: stack_ptr,
            base_stack : Some(base) ,
            state: TaskState::Ready,
            page_table: create_new_pml4(global_frame_allocator, crate::memory::PHYS_MEM_OFFSET.lock().clone()) ,
            priority: 0,
            start_time: 0,
            cpu_time: 0,
            memory_usage: STACK_SIZE,
            parent_id: None,
            children: Vec::new(),
            exit_code: None,
        }
    }
    pub fn new_boot_task() -> Self {

        let (start_frame, _) = x86_64::registers::control::Cr3::read();
        Task {
            id: NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst),
            name: "kernel_main".into(),
            stack_pointer: 0, // Will be overwritten by the first context switch
            base_stack : None ,
            state: TaskState::Running, // It is currently running
            page_table: start_frame,
            priority: 0,
            start_time: 0,
            cpu_time: 0,
            memory_usage: 0,
            parent_id: None,
            children: Vec::new(),
            exit_code: None,
        }
    }




}