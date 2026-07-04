
use core::arch::global_asm;
use crate::hlt_loop;

global_asm!(
 "   .global task_trampoline
.extern sys_exit

task_trampoline:
   # get the exit code from the stack
    mov rdi, rax


    call sys_exit


.loop:
    hlt
    jmp .loop"
);
unsafe extern "C" {

    pub fn task_trampoline();
}
#[unsafe(no_mangle)]
pub extern "C" fn sys_exit(status: u64) -> ! {


    crate::scheduler::SCHEDULER.exit_current_task(status as i32);
    crate::scheduler::SCHEDULER.schedule();
    loop {
        x86_64::instructions::interrupts::enable_and_hlt();
    }
}
