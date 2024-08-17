.global _start

_start:
    mov  r7, #4
    mov  r0, #1
    ldr  r1, =hello
    mov  r2, #13
    svc  0

    mov  r7, #1
    mov  r0, #0
    svc  0

.data
hello:    .ascii    "Hello World!\n"
