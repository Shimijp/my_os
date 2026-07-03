use alloc::vec;
use alloc::vec::Vec;
use core::arch::{global_asm};
global_asm!(include_str!("switch.s"));
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;
use crate::{hlt_loop, println};
use crate::task::Task;
    pub const MAX_TASKS: usize = 64;

    pub struct Scheduler {
        pub current_task: usize,
        pub tasks: Vec<Task>,
    }

fn idle()
{
   hlt_loop();
}
impl Scheduler {
        pub fn new() -> Self {
            let first = Task::new("init", idle);
            Scheduler {
                current_task: first.id,
                tasks: vec![first],
            }
        }

        pub fn add_task(&mut self, task: Task) -> Result<(), &'static str> {
            if self.tasks.len() >= MAX_TASKS {
                return Err("Maximum number of tasks reached");
            }
            self.tasks.push(task);
            Ok(())
        }

    /// Round-robin scheduling algorithm
        pub fn schedule(&mut self)
        {

            if self.tasks.is_empty() { return};
            let current_task: &mut Task = self.get_current_task();

            println!("was running: {}", current_task.name);
            let old_stack = &mut current_task.stack_pointer as *mut usize;
            current_task.state = crate::task::TaskState::Ready;
            let next_task = self.select_next_task();
            if next_task.name == "init" {return}
            let new_stack = next_task.stack_pointer;
            next_task.state = crate::task::TaskState::Running;
            println!("now running: {}", &next_task.name);
            self.current_task = next_task.id;
            unsafe {
                switch_task(old_stack, new_stack );
            }



        }
    pub fn select_next_task(&mut self) -> &mut Task {
        let current_index = self.tasks.iter().
            position(|t| t.id == self.current_task).unwrap();
        let len = self.tasks.len();
        for i in current_index +1  .. len {
            if self.tasks[i].state == crate::task::TaskState::Ready {
                return &mut self.tasks[i];
            }
            if self.tasks[i].state == crate::task::TaskState::New
            {
                self.tasks[i].state = crate::task::TaskState::Ready;
            }
        }
        for j in 0..=current_index {
            if self.tasks[j].state == crate::task::TaskState::Ready {
                return &mut self.tasks[j];
            }
            if self.tasks[j].state == crate::task::TaskState::New
            {
                self.tasks[j].state = crate::task::TaskState::Ready;
            }
        }
        self.tasks.get_mut(0)
            .unwrap()
        }



        pub fn get_current_task(&mut self) -> &mut Task {
            self.tasks.iter_mut().find(|t| t.id == self.current_task).unwrap()
        }
    }
unsafe extern "C" {
    pub fn switch_task(old_stack: *mut usize, new_stack: usize);
}
pub struct SchedulerWrapper {
    pub inner: Mutex<Scheduler>,
}
impl SchedulerWrapper {
    // Wraps the inner schedule method with interrupts disabled and locks
    pub fn schedule(&self) {
        without_interrupts(|| {
            let mut scheduler = self.inner.lock();
            scheduler.schedule();
        });
    }


    pub fn add_task(&self, task: Task) {
        without_interrupts(|| {
            let mut scheduler = self.inner.lock();
            scheduler.add_task(task).expect("Failed to add task"); // Assuming you have an add_task method
        });
    }


}


lazy_static! {
    // The actual global instance
    pub static ref SCHEDULER: SchedulerWrapper = SchedulerWrapper {
        inner: Mutex::new(Scheduler::new()),
    };
}