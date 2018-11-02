global long_mode_start

section .text
bits 64
long_mode_start:

    ; load 0 into all data segment registers so they don't contain the old data segment offsets
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; call the rust main
    extern rust_main        ; tell nasm that the function is defined in another file
    call rust_main

    ; print name to screen
    ; rax = g4 bit register
    ; qword = quad-word, 64 bit
    mov rax, 0xd062d061d073d049
    mov qword [0xb8000], rax
    mov rax, 0xd06cd065
    mov qword [0xb8008], rax
    hlt
