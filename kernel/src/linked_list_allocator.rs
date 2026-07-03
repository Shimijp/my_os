use core::alloc::GlobalAlloc;
use core::cmp::max;
use core::ptr;
use crate::println;

pub struct ListNode {
    pub size: usize,
    pub next:  * mut  ListNode,
}


pub struct LinkedListAllocator {
    pub dummy: ListNode,
}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        Self { dummy: ListNode { size: 0, next: ptr::null_mut() } }
    }
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.dummy.next = heap_start as *mut ListNode;
            (*self.dummy.next).size = heap_size;
            (*self.dummy.next).next = ptr::null_mut();
        }
    }

    pub fn alloc(&mut self, layout: core::alloc::Layout) -> *mut u8 {
        let size = Self::effective_size(layout);
        let align = layout.align();
        let mut current = self.dummy.next;
        let mut prev: *mut ListNode = &mut self.dummy;
        unsafe {
            while !current.is_null() {
                let current_addr = current as usize;
                let reminder = current_addr % align;
                let padding = if reminder == 0 { 0 } else { align - reminder };
                let aligned_addr = current_addr + padding;

                if (*current).size >= size + padding {
                    let addr = aligned_addr as *mut u8;
                    let left_size = (*current).size - padding - size;

                    // best case: fits perfectly, no padding, no leftover
                    if left_size == 0 && padding == 0 {
                        (*prev).next = (*current).next;
                    }
                    // fits exactly after padding — keep the padding as a shrunk node if it's big enough
                    else if left_size == 0 {
                        if padding >= size_of::<ListNode>() {
                            (*current).size = padding;
                        } else {
                            (*prev).next = (*current).next;
                        }
                    }
                    // there is some left
                    else {
                        if left_size >= size_of::<ListNode>() {
                            let new = (current_addr + padding + size) as *mut ListNode;
                            (*new).size = left_size;
                            (*new).next = (*current).next;

                            if padding >= size_of::<ListNode>() {
                                (*current).size = padding;
                                (*current).next = new;
                            } else {
                                (*prev).next = new;
                            }
                        } else {
                            // leftover too small to be a node — leak it
                            if padding >= size_of::<ListNode>() {
                                (*current).size = padding;
                            } else {
                                (*prev).next = (*current).next;
                            }
                        }
                    }

                    return addr;
                }

                prev = current;
                current = (*current).next;
            }
        }
        ptr::null_mut()
    }
    fn effective_size(layout: core::alloc::Layout) -> usize {
        let layout_size = layout.size();
        let size_reminder = layout_size % 8;
        let mut size = if size_reminder == 0 {layout_size} else { layout_size + (8 - size_reminder) };
        size = max(size, size_of::<ListNode>());
        size
    }
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: core::alloc::Layout)
    {
        let  new = ptr as *mut ListNode;
        unsafe {
            (*new).size =  Self::effective_size(layout);
            (*new).next = ptr::null_mut();
            let mut current = self.dummy.next;
            let mut prev: *mut ListNode = &mut self.dummy;
            while !current.is_null() && current < new {
                prev = &mut *current;
                current = (*current).next;
            }
            (*new).next = current;
            (*prev).next = new;



            //Coalescing with next block if they are adjacent
            if !current.is_null() && (new as usize) + (*new).size == current as usize {
                (*new).size += (*current).size;
                (*new).next = (*current).next;
            }
            let dummy_ptr: *mut ListNode = &mut self.dummy;
            if prev != dummy_ptr && (prev as usize) + (*prev).size == new as usize {
                (*prev).size += (*new).size;
                (*prev).next = (*new).next;
            }

        }

    }
}

pub struct LockedLinkedListAllocator {
    pub inner: spin::Mutex<LinkedListAllocator>,
}
impl LockedLinkedListAllocator {
    pub const fn new() -> Self {
        Self { inner: spin::Mutex::new(LinkedListAllocator::new()) }
    }
    pub unsafe fn init(&self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.inner.lock().init(heap_start, heap_size);
        }
    }
}

unsafe impl Send for LinkedListAllocator {}

unsafe impl GlobalAlloc for LockedLinkedListAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        // Lock the spinlock, then call your logic
        self.inner.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        // Lock the spinlock, then call your logic
        unsafe
            {
                self.inner.lock().dealloc(ptr, layout)
            }
    }
}
