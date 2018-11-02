; Code that the bootloader can load

;exports a label, start will be the entry point for the kernel
global start
extern long_mode_start

section .text   ;section for executable code
bits 32         ;specifies that the following lines are 32-bit instuctions, needed because the CPU is still in Protected mode when GRUB starts our kernel
start:          ; entry point that the bootloader jumps to

    ;we set the stack pointer to point at the top since the stack grows downwards
    mov esp, stack_top
    mov edi, ebx       ; Move Multiboot info pointer to edi, to pass pointer to our kernel

    call check_multiboot
    call check_cpuid
    call check_long_mode

    call set_up_page_tables
    call enable_paging

    ; load the 64-bit GDT
    lgdt [gdt64.pointer]

    jmp gdt64.code:long_mode_start

    ; dword = double word, 32 bit
;    mov dword [0xb8000], 0xd062d061d073d049     ;moves the 32-bit constant to memory at adress b8000
;    mov dword [0xb8008], 0xd06cd065
;    hlt                                         ;halt instruction and causes the CPU to stop

;make sure kernel was loaded by multiboot
;bootloader must write the magic number 0x36d76289 to eax before loading a kernel
check_multiboot:
    cmp eax, 0x36d76289     ;check if register eax is equal to the magic number
    jne .no_multiboot
    ret
.no_multiboot:
    mov al, "0"             ;sets al to error code 0
    jmp error


; CPUID - allows the software to get details about the processor
check_cpuid:
    ; Check if CPUID is supported by attempting to flip the ID bit (bit 21)
    ; in the FLAGS register. If we can flip it, CPUID is available.

    ; Copy FLAGS in to EAX via stack since we can't access it directly
    pushfd    ; save the Flag register to the stack
    pop eax   ; pop it into eax

    ; Copy to ECX as well for comparing later on
    mov ecx, eax

    ; Flip the ID bit
    xor eax, 1 << 21

    ; Copy EAX to FLAGS via the stack
    push eax  ; push eax to the stack, with the flipped bit
    popfd     ; restore flag register from eax (now with the flipped bit)

    ; Copy FLAGS back to EAX (with the flipped bit if CPUID is supported)
    pushfd    ; flag register is pushed onto the stack again
    pop eax   ; flag register is in eax

    ; Restore FLAGS from the old version stored in ECX (i.e. flipping the
    ; ID bit back if it was ever flipped).
    push ecx  ; push the old value of eax to the stack
    popfd     ; restore flag register from ecx
    ; we want the old value to be in the flag register
    ; we don't want to change the value it was from the beginning

    ; Compare EAX and ECX. If they are equal then that means the bit
    ; wasn't flipped, and CPUID isn't supported.
    cmp eax, ecx
    je .no_cpuid
    ret
.no_cpuid:
    mov al, "1"
    jmp error

;check if long mode can be used by using CPUID
check_long_mode:
    ; test if extended processor info is available
    mov eax, 0x80000000    ; let cpuid know we want to know the highest supported argument by putting 0x800000 in eax
    cpuid                  ; get highest supported argument, add result to eax
    cmp eax, 0x80000001    ; eax needs to be at least 0x80000001
    jb .no_long_mode       ; if it's less, the CPU is too old for long mode

    ; use extended info to test if long mode is available
    mov eax, 0x80000001    ; argument 0x800001 asks the cpuid for extended processor info
    cpuid                  ; returns various feature bits in ecx and edx
    test edx, 1 << 29      ; test if the LM-bit is set in the edx register
    jz .no_long_mode       ; If it's not set, there is no long mode
    ret
.no_long_mode:
    mov al, "2"
    jmp error

; Prints `ERR: ` and the given error code to screen and hangs.
; parameter: error code (in ascii) in al
error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt

set_up_page_tables:

    ; map p4 table recursively
    mov eax, p4_table
    or eax, 0b11 ; set present + writable bit
    mov [p4_table + 511 * 8], eax ; get the last entry of the p4_table (511) to point to itself

    ; map first P4 entry to first P3 table
    mov eax, p3_table
    or eax, 0b11 ; set present + writable (first two bits)
    mov [p4_table], eax ; copy eax to address of p4_table

    ; map first P3 entry to P2 table
    mov eax, p2_table
    or eax, 0b11 ; present + writable
    mov [p3_table], eax ; copy eax to address of p3_table

    ; map each P2 entry to a huge 2MiB page
    mov ecx, 0         ; counter variable

;set level two table to have valid references to pages
.map_p2_table:
    ; map ecx-th P2 entry to a huge page that starts at address 2MiB*ecx
    mov eax, 0x200000  ; 2MiB
    mul ecx            ; start address of ecx-th page
    or eax, 0b10000011 ; present + writable + huge
    mov [p2_table + ecx * 8], eax ; each entry is eight bits so we multiply counter by eight

    inc ecx            ; increase counter
    cmp ecx, 512       ; if counter == 512, the whole P2 table is mapped (we have gone through all entries)
    jne .map_p2_table  ; else map the next entry

    ret

enable_paging:
    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    ; cr3 is a control register that holds the location of the page table
    ; p4_table needs to be set in a register before we can set cr3
    mov eax, p4_table
    mov cr3, eax

    ; enable PAE-flag in cr4 (Physical Address Extension)
    ; set the fifth bit to 1 to enable
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the long mode bit in the EFER (used to enable long mode) MSR (model specific register)
    mov ecx, 0xC0000080
    rdmsr               ; read from model specific register at adress ecx and writes to eax
    or eax, 1 << 8      ; the 8 bit in EFER is the long mode
    wrmsr

    ; enable paging in the cr0 register
    ; the 31 bit in the cr0 register is the paging bit
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax
    ret

; long mode GDT
; read only data since we are not going to modufy the GDT
section .rodata
gdt64:
    dq 0 ; zero entry
.code: equ $ - gdt64
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; set the bits in the code segment

; load GDT by passing the memory location of a pointer structure
.pointer:
    dw $ - gdt64 - 1  ; specify the GDT length, $ is replaced with current address (=.pointer)
    dq gdt64          ; specify gdt address


;create a uninitialized stack
;part of the data segment, used for uninitialized objects
;we put the stack in the bss section since we do not know from the beginning how much data will be needed for it
section .bss

;addresses will be set to a multiple of 4096
align 4096

; resb - reserve bytes for the stack
p4_table:
    resb 4096
p3_table:
    resb 4096
p2_table:
    resb 4096

stack_bottom:
  resb 4096 * 4           ;reserves bytes for entry, store length of data
stack_top:
