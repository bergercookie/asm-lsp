# Sourced from https://courses.cs.vt.edu/cs2506/Fall2014/Notes/L04.MIPSAssemblyOverview.pdf
# Hello, World!
.data ## Data declaration section
## String to be printed:
out_string: .asciiz "\nHello, World!\n"
.text ## Assembly language instructions go in text segment
main: ## Start of code section
li $v0, 4 # system call code for printing string = 4
la $a0, out_string # load address of string to be printed into $a0
syscall # call operating system to perform operation
# specified in $v0
# syscall takes its arguments from $a0, $a1, ...
li $v0, 10 # terminate program
syscall
