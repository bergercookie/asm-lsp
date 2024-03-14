; Taken from page 53 of the manual https://www.zilog.com/docs/z80/z80cpu_um.pdf
        LD HL, DATA     ;STARTING ADDRESS OF DATA STRING
        LD DE, BUFFER   ;STARTING ADDRESS OF TARGET BUFFER
        LD B', 132      ;MAXIMUM STRING LENGTH
        LD A, '$'       ;STRING DELIMITER CODE
LOOP:
        CP (HL)         ;COMPARE MEMORY CONTENTS WITH
                        ;DELIMITER
        JR Z, END-$     ;GO TO END IF CHARACTERS EQUAL
        LDI             ;MOVE CHARACTER (HL) to (DE)
                        ;INCREMENT HL AND DE, DECREMENT BC
        JP PE, LOOP     ;GO TO "LOOP" IF MORE CHARACTERS
END:                    ;OTHERWISE, FALL THROUGH
;NOTE: P/V FLAG IS USED
;TO INDICATE THAT REGISTER BC WAS
;DECREMENTED TO ZERO.
