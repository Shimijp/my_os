use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};
use crate::scheduler::{block_current_task, get_current_task_id, wakeup_task, yield_now};

pub const MAX_MUTEX_WAIT_QUEUE: usize = 64;

struct TaskQueue {
    ids: [usize; MAX_MUTEX_WAIT_QUEUE],
    size: usize,
}

impl TaskQueue {
    const fn new() -> Self {
        Self { ids: [0; MAX_MUTEX_WAIT_QUEUE], size: 0 }
    }
    fn push(&mut self, id: usize) {
        self.ids[self.size] = id;
        self.size += 1;
    }
    fn pop(&mut self) -> Option<usize> {
        if self.size == 0 { return None; }
        self.size -= 1;
        Some(self.ids[self.size])
    }
    fn retain(&mut self, keep: impl Fn(usize) -> bool) {
        let mut w = 0;
        for r in 0..self.size {
            if keep(self.ids[r]) {
                self.ids[w] = self.ids[r];
                w += 1;
            }
        }
        self.size = w;
    }
}

pub struct Mutex<T> {
    state: AtomicBool,
    queue: UnsafeCell<TaskQueue>,
    data: UnsafeCell<T>,
}

// SAFETY: single-core only. All queue/data access is guarded by IF=0 + state bit.
unsafe impl<T: Send> Sync for Mutex<T> {}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

// i fucking hate rust this is so complicated why????? why????? fuck this shit
impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Mutex {
            state: AtomicBool::new(false),
            queue: UnsafeCell::new(TaskQueue::new()),
            data: UnsafeCell::new(data),
        }
    }

    fn push(&self, id: usize) {
        unsafe { (*self.queue.get()).push(id) };
    }
    fn pop(&self) -> Option<usize> {
        unsafe { (*self.queue.get()).pop() }
    }
    fn retain(&self, keep: impl Fn(usize) -> bool) {
        unsafe { (*self.queue.get()).retain(keep) };
    }
    fn queue_size(&self) -> usize {
        unsafe { (*self.queue.get()).size }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        loop {
            x86_64::instructions::interrupts::disable();
            if !self.state.swap(true, Ordering::SeqCst) {
                let current_task_id = get_current_task_id();
                self.retain(|id| id != current_task_id);
                x86_64::instructions::interrupts::enable();
                break;
            } else {
                let current_task_id = get_current_task_id();
                if self.queue_size() >= MAX_MUTEX_WAIT_QUEUE {
                    panic!("Mutex wait queue overflow");
                }
                self.push(current_task_id);
                block_current_task();
                x86_64::instructions::interrupts::enable();
                yield_now();
            }
        }
        MutexGuard { mutex: self }
    }

    pub fn unlock(&self) {
        x86_64::instructions::interrupts::disable();
        self.state.store(false, Ordering::SeqCst);
        if let Some(next_id) = self.pop() {
            wakeup_task(next_id);
        }
        x86_64::instructions::interrupts::enable();
    }
}