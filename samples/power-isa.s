loop:   lbz r4, 0(r3)
        addi r4, r4, 0x20   # 'a' - 'A'
        stb r4, 0(r3)
        addi r3, r3, 1
        b loop
