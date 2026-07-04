use x86_64::structures::paging::{PhysFrame, Size4KiB};
use x86_64::structures::paging::FrameAllocator;
use x86_64::VirtAddr;
use crate::memory::active_level_4_table;

pub fn creat_new_plm4(frame_allocator: &mut impl FrameAllocator<Size4KiB>, phys_mem_offset : VirtAddr) -> PhysFrame {
    let frame = frame_allocator.allocate_frame().expect("No more frames available");
    let pml4_adr = frame.start_address().as_u64() + phys_mem_offset.as_u64();
    let pml4_ptr = pml4_adr as *mut u64;
    unsafe {
        pml4_ptr.write_bytes(0, 512);

    }
    let page_table =  pml4_ptr as *mut x86_64::structures::paging::PageTable;
    unsafe {
        let kernel_page = active_level_4_table(phys_mem_offset);
        for i in 256 ..512 {
            (&mut (*page_table))[i] = kernel_page[i].clone();

        }

    }

    frame
}

