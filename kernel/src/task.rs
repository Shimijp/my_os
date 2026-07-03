use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
pub const STACK_SIZE: usize = 4096 * 4; // 16KB stack size
static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(0);
pub enum TaskState {
    New,
    Ready,
    Running,
    Waiting,
    Suspended,
    Zombie,
}

impl PartialEq for TaskState {
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other),
            (TaskState::New, TaskState::New) |
            (TaskState::Ready, TaskState::Ready) |
            (TaskState::Running, TaskState::Running) |
            (TaskState::Waiting, TaskState::Waiting) |
            (TaskState::Suspended, TaskState::Suspended) |
            (TaskState::Zombie, TaskState::Zombie)
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
    pub state: TaskState,
    pub page_table: usize,
    pub priority: u8,
    pub cpu_time: u64,
    pub memory_usage: usize,
    pub parent_id: Option<usize>,
    pub children: Vec<usize>,
    pub exit_code: Option<i32>,

}

impl Task {
    pub fn new(name: &str, entry_point : fn()) -> Self {
        let id = NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst);
        let layout = core::alloc::Layout::from_size_align(STACK_SIZE, 16).unwrap();
        let stack_ptr_end = unsafe { alloc::alloc::alloc(layout) as usize + STACK_SIZE };
        let stack_ptr = stack_ptr_end - size_of::<TaskContext>();
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
            let context_ptr = stack_ptr as *mut TaskContext;
            *context_ptr = context;
        }
        Task {
            id,
            name: name.into(),
            stack_pointer: stack_ptr,
            state: TaskState::New,
            page_table: 0,
            priority: 0,
            cpu_time: 0,
            memory_usage: STACK_SIZE,
            parent_id: None,
            children: Vec::new(),
            exit_code: None,
        }
    }
}