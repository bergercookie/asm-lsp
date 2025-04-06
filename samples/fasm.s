; Sourced from https://github.com/hnwfs/lin-Fasm/blob/master/samples/01/hello.asm
format ELF64 executable 3

segment readable executable

entry main

main:
    lea  rdi, [msg]                               ; address of msg goes into rdi
    mov  rax, 14                                  ; put length of msg into rax
    mov  rdx, rax                                 ; move rax to rdx
    mov  rsi, rdi                                 ; move rdi to rsi
    mov  rdi, 1                                   ; stdout
    mov  rax, 1                                   ; sys_write
    syscall
    xor  rdi, rdi                                 ; exit code 0
    mov  rax, 60                                  ; sys_exit
    syscall

segment readable writable

msg  db 'Hello world!', 10, 0
