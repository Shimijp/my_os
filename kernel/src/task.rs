use x86_64::structures::paging::{FrameDeallocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, Size4KiB};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use x86_64::structures::paging::{FrameAllocator, PhysFrame};
use x86_64::VirtAddr;
use crate::gdt::GDT;
use crate::memory::FRAME_ALLOCATOR;
use crate::pml4::{ create_new_pml4};
use crate::scheduler::SCHEDULER;

pub const STACK_SIZE: usize = 4096 * 4; // 16KB stack size
static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(0);
pub const USER_STACK_START : usize =0x0000_7000_0000_0000;
pub const USER_CODE_START: usize = 0x0000_6000_0000_0000;
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
    pub kernel_stack : usize,
    pub entry_point: u64,
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
        if self.base_stack.is_none() {return};
        let mut frame_lock = FRAME_ALLOCATOR.lock();
        let frame_allocator = frame_lock.as_mut()
            .expect("Frame allocator not initialized");
        let start_addr : u64 = USER_STACK_START as u64;
        let phys_start =crate::memory::PHYS_MEM_OFFSET.lock().clone();
        let page_table = self.page_table;
        let mapper_addr = (page_table.start_address().as_u64() + phys_start.as_u64() )as * mut PageTable;
        let mut mapper = unsafe{OffsetPageTable::new(&mut *mapper_addr, phys_start)};
        for i in 1 ..=4
        {
            let virtual_address = VirtAddr::new(start_addr - (i * 4096));
            let page : Page<Size4KiB> = Page::containing_address(virtual_address);

            let (frame, flush) = unsafe  { mapper.unmap(page)
                .expect("failed to map page table")};
            flush.flush();
            unsafe { frame_allocator.deallocate_frame(frame); }


        }
        unsafe {frame_allocator.deallocate_frame(self.page_table)}
        let layout = core::alloc::Layout::from_size_align(STACK_SIZE, 16).unwrap();
        let kernel_base = self.kernel_stack - STACK_SIZE;
        unsafe {alloc::alloc::dealloc(kernel_base as *mut u8, layout)}

    }
}
impl Task {
    pub fn new(name: &str, entry_point : fn() -> u64) -> Self {
        let id = NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst);



        const USER_LOOP: [u8; 2] = [0xEB, 0xFE];
        let stack_base = Self::create_mem(&USER_LOOP);
        let kernel_stack_start = core::alloc::Layout::from_size_align(STACK_SIZE, 16).unwrap();
        let kernel_stack = unsafe {alloc::alloc::alloc(kernel_stack_start) as usize } + STACK_SIZE;



        // Set up the stack for the new task

        let context = TaskContext {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            rflags: 0x202, // Default RFLAGS value
            rip: trampoline as *const () as u64,        // Set to the entry point of the task
        };
        let stack_ptr = (kernel_stack - size_of::<TaskContext>() - size_of::<usize>()) as *mut TaskContext;
        unsafe {core::ptr::write(stack_ptr, context)};


        Task {
            id,
            name: name.into(),
            stack_pointer: stack_ptr as usize,
            base_stack : Some(USER_STACK_START) ,
            kernel_stack,
            entry_point: USER_CODE_START as u64,
            state: TaskState::Ready,
            page_table: stack_base ,
            priority: 0,
            start_time: 0,
            cpu_time: 0,
            memory_usage: STACK_SIZE,
            parent_id: None,
            children: Vec::new(),
            exit_code: None,
        }
    }
    fn create_mem(entry_code: &[u8])-> PhysFrame{
        let mut frame_lock = FRAME_ALLOCATOR.lock();
        let frame_allocator = frame_lock.as_mut()
            .expect("Frame allocator not initialized");
        let start_addr : u64 = 0x0000_7000_0000_0000;
        let phys_start =crate::memory::PHYS_MEM_OFFSET.lock().clone();
        let page_table = create_new_pml4(frame_allocator,phys_start);
        let mapper_addr = (page_table.start_address().as_u64() + phys_start.as_u64()) as * mut PageTable;
        let mut mapper = unsafe{OffsetPageTable::new(&mut *mapper_addr, phys_start)};
        for i in 1 ..=4
        {
            let virtual_address = VirtAddr::new(start_addr - (i * 4096));
            let page = Page::containing_address(virtual_address);
            let frame = frame_allocator.allocate_frame().unwrap();
            let flags =  PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
            let flush =unsafe  { mapper.map_to(page, frame, flags, frame_allocator)
                .expect("failed to map page table")};
            flush.flush();



        }
        //code frame
        let code_addr: u64 = 0x0000_6000_0000_0000;
        let code_page = Page::containing_address(VirtAddr::new(code_addr));
        let code_frame = frame_allocator.allocate_frame().unwrap();
        let flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
        let flush = unsafe {
            mapper.map_to(code_page, code_frame, flags, frame_allocator)
                .expect("failed to map code page")
        };
        flush.flush();

        let dst = (code_frame.start_address().as_u64() + phys_start.as_u64()) as *mut u8;
        unsafe { core::ptr::copy_nonoverlapping(entry_code.as_ptr(), dst, entry_code.len()); }

        page_table
    }
    pub fn new_boot_task() -> Self {

        let (start_frame, _) = x86_64::registers::control::Cr3::read();
        Task {
            id: NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst),
            name: "kernel_main".into(),
            stack_pointer: 0, // Will be overwritten by the first context switch
            base_stack : None ,
            entry_point: 0,
            kernel_stack : 0,
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
pub extern "C" fn trampoline()
{
    let mut scheduler_guard = SCHEDULER.inner.lock();
    let current_task = scheduler_guard.get_current_task();
    let user_code_segment = GDT.1.user_code_selector;
    let user_data_segment = GDT.1.user_data_selector;
    let user_entry = current_task.entry_point;
    let base_stack = current_task.base_stack;
    drop(scheduler_guard);
    let cs = user_code_segment.0 as u64 | 3;
    let ss = user_data_segment.0 as u64 | 3;
    unsafe {
        core::arch::asm!(
            "push {ss}",
            "push {rsp}",
            "push {rflags}",
            "push {cs}",
            "push {rip}",
            "iretq",
            ss     = in(reg) ss,
            rsp    = in(reg) base_stack.unwrap(),
            rflags = in(reg) 0x202u64,
            cs     = in(reg) cs,
            rip    = in(reg) user_entry,
            options(noreturn)

        )
    }

}