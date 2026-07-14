    use alloc::vec;
    use alloc::vec::Vec;
    use core::arch::{asm, global_asm};
    use core::sync::atomic::AtomicBool;

    global_asm!(include_str!("switch.s"));
    use lazy_static::lazy_static;
    use spin::Mutex;
    use x86_64::instructions::interrupts::without_interrupts;
    use crate::println;

    pub static HAS_TERMINATED_TASKS: AtomicBool = AtomicBool::new(false);

    use crate::task::Task;
    pub const MAX_TASKS: usize = 64;

    pub struct Scheduler {
        pub init_id: usize,
        pub current_task: usize,
        pub tasks: Vec<Task>,
    }


    impl Scheduler {
        pub fn new() -> Self {
            let first = Task::new_boot_task();
            Scheduler {
                init_id: first.id,
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
        pub fn prepare_schedule(&mut self) -> Option<(*mut usize, usize, x86_64::structures::paging::PhysFrame)> {

            let current_task_id = self.current_task;
            let current_task: &mut Task = self.get_current_task();


            unsafe {
                current_task.cpu_time += core::arch::x86_64::_rdtsc() - current_task.start_time;
            }
            //
            let old_stack = &mut current_task.stack_pointer as *mut usize;
            if current_task.state == crate::task::TaskState::Running {
                current_task.state = crate::task::TaskState::Ready;
            }
            let next_task = self.select_next_task();
            let next_task_id = next_task.id;
            if current_task_id == next_task_id {
                return None;
            }
            let page_table = next_task.page_table;
            let new_stack = next_task.stack_pointer;
            next_task.state = crate::task::TaskState::Running;
            self.current_task = next_task.id;
            Some((old_stack, new_stack, page_table))
        }
        pub fn clear_terminated_tasks(&mut self) {
            self.tasks.retain(|task| task.state != crate::task::TaskState::Terminated && task.state != crate::task::TaskState::Zombie);
            HAS_TERMINATED_TASKS.swap(false, core::sync::atomic::Ordering::SeqCst);

        }

        pub fn select_next_task(&mut self) -> &mut Task {

            let current_index = self.tasks.iter().
                position(|t| t.id == self.current_task).unwrap();
            let len = self.tasks.len();
            for i in current_index + 1..len {
                if self.tasks[i].state == crate::task::TaskState::Ready {
                    return &mut self.tasks[i];
                }
            }
            for j in 0..=current_index {
                if self.tasks[j].state == crate::task::TaskState::Ready {
                    return &mut self.tasks[j];
                }
            }
            self.tasks.get_mut(0)
                .unwrap()
        }

        pub fn exit_current_task(&mut self, exit_code: i32) {
            self.exit_task(self.current_task, exit_code);
        }
        fn exit_task(&mut self, task_id: usize, exit_code: i32) {
            if task_id == self.init_id {
                panic!("Init task cannot be terminated");
            }
            if let Some(task_index) = self.tasks.iter().position(|t| t.id == task_id) {
                let task = &mut self.tasks[task_index];

                unsafe
                    {
                        HAS_TERMINATED_TASKS.swap(true, core::sync::atomic::Ordering::SeqCst);
                        task.cpu_time += core::arch::x86_64::_rdtsc() - task.start_time;
                    }
                task.state = crate::task::TaskState::Terminated;
                task.exit_code = Some(exit_code);
            }
        }



        pub fn get_current_task(&mut self) -> &mut Task {
            self.tasks.iter_mut().find(|t| t.id == self.current_task).unwrap()
        }
        pub fn block_current_task(&mut self) {
            let current_task = self.get_current_task();
            current_task.state = crate::task::TaskState::Blocked;
            unsafe
                {
                    current_task.cpu_time += core::arch::x86_64::_rdtsc() - current_task.start_time;
                }
        }
        pub fn wakeup_task(&mut self, task_id: usize) {
            if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
                if task.state == crate::task::TaskState::Blocked {
                    task.state = crate::task::TaskState::Ready;
                }
            }
        }
    }
    pub fn yield_now()
    {

        unsafe

            {
                asm!{
                "int 0x81",

                }
            }
    }
    pub fn block_current_task() {
        SCHEDULER.block_current_task();
    }
    pub fn wakeup_task(task_id: usize) {
        SCHEDULER.wakeup_task(task_id);
    }
    pub fn get_current_task_id() -> usize {
        SCHEDULER.get_current_task_id()
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
                let pointers = {
                    let mut scheduler = self.inner.lock();
                    scheduler.prepare_schedule()
                }; // ← ה-lock מת כאן — אין deadlock

                if let Some((old_stack, new_stack, next_pt)) = pointers {
                    let (cur_pt, flags) = x86_64::registers::control::Cr3::read();
                    if cur_pt != next_pt {
                        unsafe { x86_64::registers::control::Cr3::write(next_pt, flags); }
                    }
                    unsafe { switch_task(old_stack, new_stack); }
                }
            });
        }

        pub fn exit_current_task(&self, exit_code: i32) {
            without_interrupts(|| {
                let mut scheduler = self.inner.lock();
                scheduler.exit_current_task(exit_code);

            });
        }
        pub fn block_current_task(&self) {
            without_interrupts(|| {
                let mut scheduler = self.inner.lock();
                scheduler.block_current_task();
            });
        }
        pub fn wakeup_task(&self, task_id: usize) {
            without_interrupts(|| {
                let mut scheduler = self.inner.lock();
                scheduler.wakeup_task(task_id);
            });
        }
        pub fn clear_terminated_tasks(&self) {
            without_interrupts(|| {
                let mut scheduler = self.inner.lock();
                scheduler.clear_terminated_tasks();

            });
        }
        pub fn get_current_task_id(&self) -> usize {
            without_interrupts(|| {
                let scheduler = self.inner.lock();
                scheduler.current_task
            })
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