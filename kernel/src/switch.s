.global switch_task
switch_task:
    # Save callee-saved registers
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15


    # Save CPU flags
    pushfq

    # Switch stacks
    mov [rdi], rsp
    mov rsp, rsi

    # Restore CPU flags
    popfq

    # Restore callee-saved registers
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx

    # Return to the new task
    ret