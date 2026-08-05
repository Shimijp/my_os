use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use lazy_static::lazy_static;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use crate::mutex::Mutex;
use crate::println;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
struct Selectors {
    data_selector: SegmentSelector,
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
    user_data_selector : SegmentSelector,
    user_code_selector: SegmentSelector

}


lazy_static!
{
    static ref TSS : TaskStateSegment =
    {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
         {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start =  VirtAddr::from_ptr(&raw const STACK);
            let stack_end = stack_start + STACK_SIZE as u64;
            stack_end
        };
        tss

    };
}
lazy_static!
{
     static ref GDT :( GlobalDescriptorTable , Selectors)=
    {
        let mut gdt = GlobalDescriptorTable::new();
        let data_selector = gdt.append(Descriptor::kernel_data_segment());
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let tss_selector  = gdt.append(Descriptor::tss_segment(&TSS));
        let user_data_selector = gdt.append(Descriptor::user_data_segment());
        let user_code_selector = gdt.append(Descriptor::user_code_segment());
        (gdt, Selectors{data_selector, code_selector, tss_selector , user_data_selector,user_code_selector})
    };
}
pub fn set_kernel_stack(stack: usize)
{
    let tss = &*TSS;
    let tss_ptr = &*TSS as *const TaskStateSegment as *mut TaskStateSegment;;
    let stack_virt = VirtAddr::new(stack as u64);
    unsafe {(*tss_ptr).privilege_stack_table[0]= stack_virt};
}
pub fn init_gdt()
{
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS, SS,Segment};


    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        SS::set_reg(GDT.1.data_selector);
        load_tss(GDT.1.tss_selector);

    }
}
