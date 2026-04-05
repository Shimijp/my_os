use x86_64::{structures::paging::PageTable, PhysAddr, VirtAddr};
use x86_64::structures::paging::OffsetPageTable;

use crate::println;

pub unsafe fn active_level_4_table(phys_mem_offset: VirtAddr) ->&'static mut PageTable
{
    use x86_64::registers::control::Cr3;
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = phys_mem_offset + phys.as_u64()  ;
    let page_table_ptr : *mut PageTable = virt.as_mut_ptr();
    unsafe  {&mut *page_table_ptr}

}



pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe
        {
            let level_4_table = active_level_4_table(physical_memory_offset);
            OffsetPageTable::new(level_4_table, physical_memory_offset)
        }
}
