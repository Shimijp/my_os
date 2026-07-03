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
    pub fn new(name: &str) -> Self {
        let id = NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst);
        let layout = core::alloc::Layout::from_size_align(STACK_SIZE, 16).unwrap();
        let stack_ptr = unsafe { alloc::alloc::alloc(layout) as usize + STACK_SIZE };
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