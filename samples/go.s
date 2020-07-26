0x0000 00000 (x.go:3)	TEXT	"".main(SB), $16-0
0x0000 00000 (x.go:3)	MOVQ	(TLS), CX
0x0009 00009 (x.go:3)	CMPQ	SP, 16(CX)
0x000d 00013 (x.go:3)	JLS	67
0x000f 00015 (x.go:3)	SUBQ	$16, SP
0x0013 00019 (x.go:3)	MOVQ	BP, 8(SP)
0x0018 00024 (x.go:3)	LEAQ	8(SP), BP
0x001d 00029 (x.go:3)	FUNCDATA	$0, gclocals·33cdeccccebe80329f1fdbee7f5874cb(SB)
0x001d 00029 (x.go:3)	FUNCDATA	$1, gclocals·33cdeccccebe80329f1fdbee7f5874cb(SB)
0x001d 00029 (x.go:3)	FUNCDATA	$2, gclocals·33cdeccccebe80329f1fdbee7f5874cb(SB)
0x001d 00029 (x.go:4)	PCDATA	$0, $0
0x001d 00029 (x.go:4)	PCDATA	$1, $0
0x001d 00029 (x.go:4)	CALL	runtime.printlock(SB)
0x0022 00034 (x.go:4)	MOVQ	$3, (SP)
0x002a 00042 (x.go:4)	CALL	runtime.printint(SB)
0x002f 00047 (x.go:4)	CALL	runtime.printnl(SB)
0x0034 00052 (x.go:4)	CALL	runtime.printunlock(SB)
0x0039 00057 (x.go:5)	MOVQ	8(SP), BP
0x003e 00062 (x.go:5)	ADDQ	$16, SP
0x0042 00066 (x.go:5)	RET
0x0043 00067 (x.go:5)	NOP
0x0043 00067 (x.go:3)	PCDATA	$1, $-1
0x0043 00067 (x.go:3)	PCDATA	$0, $-1
0x0043 00067 (x.go:3)	CALL	runtime.morestack_noctxt(SB)
0x0048 00072 (x.go:3)	JMP	0
