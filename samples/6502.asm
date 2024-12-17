; Sourced from https://github.com/cbpmckinney/csc371-s23
; Example of keyboard input with assembly

* = $0801 ; SYS 4096 wedge
.byte $08, $0d, $0a, $00, $9e, $28, $34
.byte $30, $39, $36, $29, $00, $00, $00

* = $1000

SCINIT = $FF81  ; SCINIT KERNAL ROUTINE
PLOT = $FFF0    ; PLOT KERNAL ROUTINE
CHRIN = $FFCF   ; CHRIN KERNAL ROUTINE
CHROUT = $FFD2  ; CHROUT KERNAL ROUTINE


jsr SCINIT      ; clears screen

clc
ldx #0
ldy #0
jsr PLOT        ;resets cursor to top left of screen

main
jsr CHRIN
sta inputnumber, x
lda inputnumber, x
sec
sbc #$30
sta inputnumber, x
inx
jmp main

inputnumber.byte $00


SETCURSOR .macro
    clc
    ldx #\1
    ldy #\2
    jsr PLOT
    .endmac
