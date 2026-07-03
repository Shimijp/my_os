use alloc::vec;
use alloc::vec::Vec;
use core::cmp::PartialEq;
use crate::task::Task;
    pub const MAX_TASKS: usize = 64;

    pub struct Scheduler {
        pub current_task: usize,
        pub tasks: Vec<Task>,
    }


impl Scheduler {
        pub fn new() -> Self {
            let first = Task::new("init");
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
        pub fn schedule( &self) -> &Task
        {

            let current_index = self.tasks.iter().
                position(|t| t.id == self.current_task).unwrap();
            let len = self.tasks.len();
            for i in current_index .. len {
                if self.tasks[i].state == crate::task::TaskState::Ready {
                    return &self.tasks[i];
                }
            }
            for j in 0..current_index {
                if self.tasks[j].state == crate::task::TaskState::Ready {
                    return &self.tasks[j];
                }
            }
            self.tasks.get(0)
                .unwrap()
        }



        pub fn get_current_task(&self) -> &Task {
            self.tasks.get(self.current_task)
                .expect("Current task index out of bounds")
        }
    }