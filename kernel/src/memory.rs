use bootloader_api;
use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use x86_64::structures::paging::{OffsetPageTable, PageTable};
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{FrameAllocator, Mapper, Page, PhysFrame, Size4KiB},
};

pub struct BootInfoFrameAllocator {
    memory_region: &'static MemoryRegions,
    next: usize,
}
unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_region: &'static MemoryRegions) -> Self {
        BootInfoFrameAllocator {
            memory_region,
            next: 0,
        }
    }
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_region.iter();
        let usable_regions = regions.filter(|r| r.kind == MemoryRegionKind::Usable);
        let addr_range = usable_regions.map(|r| r.start..r.end);
        let frame_addr = addr_range.flat_map(|r| r.step_by(4096));
        frame_addr.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}
pub unsafe fn active_level_4_table(phys_mem_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = phys_mem_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    unsafe { &mut *page_table_ptr }
}

pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}
pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    use x86_64::structures::paging::PageTableFlags as Flags;
    let frame =
        PhysFrame::containing_address(PhysAddr::new(PhysAddr::new(0xb8000u64).as_u64()));
    let flags = Flags::PRESENT | Flags::WRITABLE;
    let map_to_res = unsafe { mapper.map_to(page, frame, flags, frame_allocator) };
    map_to_res.expect("map failed").flush();
}
