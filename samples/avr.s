; Sourced from https://www.instructables.com/Command-Line-Assembly-Language-Programming-for-Ard-2/

.device ATmega328P
.org 0x0000
jmp a
.org 0x0020
jmp e
a:
   ldi r16,0x05
   out 0x25,r16
   ldi r16,0x01
   sts 0x6e,r16
   sei
   clr r16
   out 0x26,r16
   sbi 0x0a,0x04
   sbi 0x0b,0x04
b:
   sbi 0x0b,0x04
   rcall c
   cbi 0x0b,0x04
   rcall c
   rjmp b
c:
   clr r17
d:
   cpi r17,0x1e
   brne d
   ret
e:
   inc r17
   cpi r17, 0x3d
   brne PC+2
   clr r17
   reti
