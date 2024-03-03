    .global Fr_rawAdd
    .global Fr_rawAddLS
    .global Fr_rawSub
    .global Fr_rawSubRegular
    .global Fr_rawNeg
    .global Fr_rawNegLS
    .global Fr_rawSubSL
    .global Fr_rawSubLS
    .global Fr_rawMMul
    .global Fr_rawMMul1
    .global Fr_rawFromMontgomery
    .global Fr_rawCopy
    .global Fr_rawSwap
    .global Fr_rawIsEq
    .global Fr_rawIsZero
    .global Fr_rawCopyS2L
    .global Fr_rawCmp
    .global Fr_rawAnd
    .global Fr_rawOr
    .global Fr_rawXor
    .global Fr_rawShr
    .global Fr_rawShl
    .global Fr_rawNot

    .global _Fr_rawAdd
    .global _Fr_rawAddLS
    .global _Fr_rawSub
    .global _Fr_rawSubRegular
    .global _Fr_rawNeg
    .global _Fr_rawNegLS
    .global _Fr_rawSubSL
    .global _Fr_rawSubLS
    .global _Fr_rawMMul
    .global _Fr_rawMMul1
    .global _Fr_rawFromMontgomery
    .global _Fr_rawCopy
    .global _Fr_rawSwap
    .global _Fr_rawIsEq
    .global _Fr_rawIsZero
    .global _Fr_rawCopyS2L
    .global _Fr_rawCmp
    .global _Fr_rawAnd
    .global _Fr_rawOr
    .global _Fr_rawXor
    .global _Fr_rawShr
    .global _Fr_rawShl
    .global _Fr_rawNot

    .text
    .align 4

// void Fr_rawAdd(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB)
Fr_rawAdd:
_Fr_rawAdd:
        ldp  x3, x4, [x1]
        ldp  x7, x8, [x2]
        adds x3, x3, x7
        adcs x4, x4, x8

        ldp  x5, x6,  [x1, 16]
        ldp  x9, x10, [x2, 16]
        adcs x5, x5, x9
        adcs x6, x6, x10

        cset x16, cs

        adr x11, Fr_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        subs x7,  x3, x12
        sbcs x8,  x4, x13
        sbcs x9,  x5, x14
        sbcs x10, x6, x15

        cbnz x16, Fr_rawAdd_done_s
        b.hs      Fr_rawAdd_done_s

        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret

Fr_rawAdd_done_s:
        stp x7, x8,  [x0]
        stp x9, x10, [x0, 16]
        ret


//void Fr_rawAddLS(FrRawElement pRawResult, FrRawElement pRawA, uint64_t rawB)
Fr_rawAddLS:
_Fr_rawAddLS:
        ldp  x3, x4, [x1]
        adds x3, x3, x2
        adcs x4, x4, xzr

        ldp  x5, x6,  [x1, 16]
        adcs x5, x5, xzr
        adcs x6, x6, xzr

        cset x16, cs

        adr x11, Fr_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        subs x7,  x3, x12
        sbcs x8,  x4, x13
        sbcs x9,  x5, x14
        sbcs x10, x6, x15

        cbnz x16, Fr_rawAddLS_done_s
        b.hs      Fr_rawAddLS_done_s

        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret

Fr_rawAddLS_done_s:
        stp x7, x8,  [x0]
        stp x9, x10, [x0, 16]
        ret


// void Fr_rawSub(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB)
Fr_rawSub:
_Fr_rawSub:
        ldp  x3, x4, [x1]
        ldp  x7, x8, [x2]
        subs x3, x3, x7
        sbcs x4, x4, x8

        ldp  x5, x6,  [x1, 16]
        ldp  x9, x10, [x2, 16]
        sbcs x5, x5, x9
        sbcs x6, x6, x10

        b.cs Fr_rawSub_done

        adr x11, Fr_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        adds x3, x3, x12
        adcs x4, x4, x13
        adcs x5, x5, x14
        adc  x6, x6, x15

Fr_rawSub_done:
        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret


//void Fr_rawSubRegular(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB)
Fr_rawSubRegular:
_Fr_rawSubRegular:
        ldp  x3, x4, [x1]
        ldp  x7, x8, [x2]
        subs x3, x3, x7
        sbcs x4, x4, x8

        ldp  x5, x6,  [x1, 16]
        ldp  x9, x10, [x2, 16]
        sbcs x5, x5, x9
        sbc  x6, x6, x10

        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret

//void Fr_rawSubSL(FrRawElement pRawResult, uint64_t rawA, FrRawElement pRawB)
Fr_rawSubSL:
_Fr_rawSubSL:
        ldp  x7, x8, [x2]
        subs x3, x1,  x7
        sbcs x4, xzr, x8

        ldp  x9, x10, [x2, 16]
        sbcs x5, xzr, x9
        sbcs x6, xzr, x10

        b.cs Fr_rawSubSL_done

        adr x11, Fr_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        adds x3, x3, x12
        adcs x4, x4, x13
        adcs x5, x5, x14
        adc  x6, x6, x15

Fr_rawSubSL_done:
        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret


//void Fr_rawSubLS(FrRawElement pRawResult, FrRawElement pRawA, uint64_t rawB)
Fr_rawSubLS:
_Fr_rawSubLS:
        ldp  x3, x4, [x1]
        subs x3, x3, x2
        sbcs x4, x4, xzr

        ldp  x5, x6,  [x1, 16]
        sbcs x5, x5, xzr
        sbcs x6, x6, xzr

        b.cs Fr_rawSubLS_done

        adr x11, Fr_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        adds x3, x3, x12
        adcs x4, x4, x13
        adcs x5, x5, x14
        adc  x6, x6, x15

Fr_rawSubLS_done:
        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret


// void Fr_rawNeg(FrRawElement pRawResult, FrRawElement pRawA)
Fr_rawNeg:
_Fr_rawNeg:
        ldp x2, x3, [x1]
        orr x6, x2, x3

        ldp x4, x5, [x1, 16]
        orr x7, x4, x5
        orr x8, x6, x7

        cbz x8, Fr_rawNeg_done_zero

        adr x10, Fr_rawq
        ldp x11, x12, [x10]
        ldp x13, x14, [x10, 16]

        subs x2, x11, x2
        sbcs x3, x12, x3
        sbcs x4, x13, x4
        sbc  x5, x14, x5

        stp x2, x3, [x0]
        stp x4, x5, [x0, 16]
        ret

Fr_rawNeg_done_zero:
        stp xzr, xzr, [x0]
        stp xzr, xzr, [x0, 16]
        ret


//void Fr_rawNegLS(FrRawElement pRawResult, FrRawElement pRawA, uint64_t rawB)
Fr_rawNegLS:
_Fr_rawNegLS:
        ldp x3, x4, [x1]
        ldp x5, x6, [x1, 16]

        adr x11, Fr_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        subs x7,  x12, x2
        sbcs x8,  x13, xzr
        sbcs x9,  x14, xzr
        sbcs x10, x15, xzr

        cset x16, cs

        subs x7,  x7,  x3
        sbcs x8,  x8,  x4
        sbcs x9,  x9,  x5
        sbcs x10, x10, x6

        cset x17, cs
        orr  x17, x17, x16

        cbz x17, Fr_rawNegLS_done

        adds x7,  x7,  x12
        adcs x8,  x8,  x13
        adcs x9,  x9,  x14
        adc  x10, x10, x15


Fr_rawNegLS_done:
        stp x7, x8,  [x0]
        stp x9, x10, [x0, 16]
        ret


// void Fr_rawMMul(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB)
Fr_rawMMul:
_Fr_rawMMul:
        ldr x3,     [x1]    //pRawA[0]
        ldp x5, x6, [x2]    //pRawB
        ldp x7, x8, [x2, 16]

        adr x4, Fr_np
        ldr x4, [x4]

        str x28, [sp, #-16]!

        adr x2, Fr_rawq
        ldp x15, x16, [x2]
        ldp x17, x28, [x2, 16]

        // product0 = pRawB * pRawA[0]
        mul   x10, x5, x3
        umulh x11, x5, x3
        mul   x2,  x6, x3
        adds  x11, x11, x2
        umulh x12, x6, x3
        mul   x2,  x7, x3
        adcs  x12, x12, x2
        umulh x13, x7, x3
        mul   x2,  x8, x3
        adcs  x13, x13, x2
        umulh x14, x8, x3
        adc   x14, x14, xzr

        // np0 = Fr_np * product0[0];
        mul   x9, x4, x10

        // product0 = product0 + Fr_rawq * np0
        mul   x2,  x15, x9
        adds  x10, x10, x2
        mul   x3,  x16, x9
        adcs  x11, x11, x3
        mul   x2,  x17, x9
        adcs  x12, x12, x2
        mul   x3,  x28, x9
        adcs  x13, x13, x3
        adc   x14, x14, xzr

        umulh x2,  x15, x9
        adds  x11, x11, x2
        umulh x3,  x16, x9
        adcs  x12, x12, x3
        umulh x2,  x17, x9
        adcs  x13, x13, x2
        umulh x3,  x28, x9
        adcs  x14, x14, x3
        adc   x9,  xzr, xzr

        // product1 = product0 + pRawB * pRawA[1]
        ldr x3, [x1, 8]    //pRawA[1]
        mul   x10, x5,  x3
        adds  x10, x10, x11
        mul   x11, x6,  x3
        adcs  x11, x11, x12
        mul   x12, x7,  x3
        adcs  x12, x12, x13
        mul   x13, x8,  x3
        adcs  x13, x13, x14
        adc   x14, xzr, xzr

        adds  x11, x11, x9
        umulh x2,  x5,  x3
        adcs  x11, x11, x2
        umulh x9,  x6,  x3
        adcs  x12, x12, x9
        umulh x2,  x7,  x3
        adcs  x13, x13, x2
        umulh x9,  x8,  x3
        adc   x14, x14, x9

        // np0 = Fr_np * product1[0];
        mul   x9, x4, x10

        // product1 = product1 + Fr_rawq * np0
        mul   x2,  x15, x9
        adds  x10, x10, x2
        mul   x3,  x16, x9
        adcs  x11, x11, x3
        mul   x2,  x17, x9
        adcs  x12, x12, x2
        mul   x3,  x28, x9
        adcs  x13, x13, x3
        adc   x14, x14, xzr

        umulh x2,  x15, x9
        adds  x11, x11, x2
        umulh x3,  x16, x9
        adcs  x12, x12, x3
        umulh x2,  x17, x9
        adcs  x13, x13, x2
        umulh x3,  x28, x9
        adcs  x14, x14, x3
        adc   x9,  xzr, xzr


        // product2 = product1 + pRawB * pRawA[2]
        ldr x3, [x1, 16]    //pRawA[2]
        mul   x10, x5,  x3
        adds  x10, x10, x11
        mul   x11, x6,  x3
        adcs  x11, x11, x12
        mul   x12, x7,  x3
        adcs  x12, x12, x13
        mul   x13, x8,  x3
        adcs  x13, x13, x14
        adc   x14, xzr, xzr

        adds  x11, x11, x9
        umulh x2,  x5,  x3
        adcs  x11, x11, x2
        umulh x9,  x6,  x3
        adcs  x12, x12, x9
        umulh x2,  x7,  x3
        adcs  x13, x13, x2
        umulh x9,  x8,  x3
        adc   x14, x14, x9

        // np0 = Fr_np * product2[0];
        mul   x9, x4, x10

        // product2 = product2 + Fr_rawq * np0
        mul   x2,  x15, x9
        adds  x10, x10, x2
        mul   x3,  x16, x9
        adcs  x11, x11, x3
        mul   x2,  x17, x9
        adcs  x12, x12, x2
        mul   x3,  x28, x9
        adcs  x13, x13, x3
        adc   x14, x14, xzr

        umulh x2,  x15, x9
        adds  x11, x11, x2
        umulh x3,  x16, x9
        adcs  x12, x12, x3
        umulh x2,  x17, x9
        adcs  x13, x13, x2
        umulh x3,  x28, x9
        adcs  x14, x14, x3
        adc   x9,  xzr, xzr

        // product3 = product2 + pRawB * pRawA[3]
        ldr x3, [x1, 24]    //pRawA[3]
        mul   x10, x5,  x3
        adds  x10, x10, x11
        mul   x11, x6,  x3
        adcs  x11, x11, x12
        mul   x12, x7,  x3
        adcs  x12, x12, x13
        mul   x13, x8,  x3
        adcs  x13, x13, x14
        adc   x14, xzr, xzr

        adds  x11, x11, x9
        umulh x2,  x5,  x3
        adcs  x11, x11, x2
        umulh x9,  x6,  x3
        adcs  x12, x12, x9
        umulh x2,  x7,  x3
        adcs  x13, x13, x2
        umulh x9,  x8,  x3
        adc   x14, x14, x9

        // np0 = Fr_np * product3[0];
        mul   x9, x4, x10

        // product3 = product3 + Fr_rawq * np0
        mul   x2,  x15, x9
        adds  x10, x10, x2
        mul   x3,  x16, x9
        adcs  x11, x11, x3
        mul   x2,  x17, x9
        adcs  x12, x12, x2
        mul   x3,  x28, x9
        adcs  x13, x13, x3
        adc   x14, x14, xzr

        umulh x2,  x15, x9
        adds  x11, x11, x2
        umulh x3,  x16, x9
        adcs  x12, x12, x3
        umulh x2,  x17, x9
        adcs  x13, x13, x2
        umulh x3,  x28, x9
        adcs  x14, x14, x3

        // result >= Fr_rawq
        subs x5, x11, x15
        sbcs x6, x12, x16
        sbcs x7, x13, x17
        sbcs x8, x14, x28

        ldr x28, [sp], #16

        b.hs Fr_rawMul_done_s

        stp x11, x12, [x0]
        stp x13, x14, [x0, 16]
        ret

Fr_rawMul_done_s:
        stp x5, x6, [x0]
        stp x7, x8, [x0, 16]
        ret


// void Fr_rawMMul1(FrRawElement pRawResult, FrRawElement pRawA, uint64_t pRawB)
Fr_rawMMul1:
_Fr_rawMMul1:
        ldp x5, x6, [x1]    //pRawA
        ldp x7, x8, [x1, 16]

        adr x4, Fr_np
        ldr x4, [x4]

        // product0 = pRawA * pRawB
        mul   x10, x5, x2
        umulh x11, x5, x2
        mul   x3,  x6, x2
        adds  x11, x11, x3
        umulh x12, x6, x2
        mul   x3,  x7, x2
        adcs  x12, x12, x3
        umulh x13, x7, x2
        mul   x3,  x8, x2
        adcs  x13, x13, x3
        umulh x14, x8, x2
        adc   x14, x14, xzr

        adr x3, Fr_rawq
        ldp x15, x16, [x3]
        ldp x17, x8,  [x3, 16]

        // np0 = Fr_np * product0[0];
        mul   x9, x4, x10

        // product0 = product0 + Fr_rawq * np0
        mul   x2,  x15, x9
        adds  x10, x10, x2
        mul   x3,  x16, x9
        adcs  x11, x11, x3
        mul   x2,  x17, x9
        adcs  x12, x12, x2
        mul   x3,  x8,  x9
        adcs  x13, x13, x3
        adc   x14, x14, xzr

        umulh x2,  x15, x9
        adds  x11, x11, x2
        umulh x3,  x16, x9
        adcs  x12, x12, x3
        umulh x2,  x17, x9
        adcs  x13, x13, x2
        umulh x3,  x8,  x9
        adcs  x14, x14, x3
        adc   x7,  xzr, xzr

        // np0 = Fr_np * product1[0];
        mul   x9, x4, x11

        // product1 = product1 + Fr_rawq * np0
        mul   x2,  x15, x9
        adds  x10, x11, x2
        mul   x3,  x16, x9
        adcs  x11, x12, x3
        mul   x2,  x17, x9
        adcs  x12, x13, x2
        mul   x3,  x8,  x9
        adcs  x13, x14, x3
        adc   x14, xzr, xzr

        adds x11,  x11, x7
        umulh x2,  x15, x9
        adcs  x11, x11, x2
        umulh x3,  x16, x9
        adcs  x12, x12, x3
        umulh x2,  x17, x9
        adcs  x13, x13, x2
        umulh x3,  x8,  x9
        adcs  x14, x14, x3
        adc   x7,  xzr, xzr

        // np0 = Fr_np * product2[0];
        mul   x9, x4, x11

        // product2 = product2 + Fr_rawq * np0
        mul   x2,  x15, x9
        adds  x10, x11, x2
        mul   x3,  x16, x9
        adcs  x11, x12, x3
        mul   x2,  x17, x9
        adcs  x12, x13, x2
        mul   x3,  x8,  x9
        adcs  x13, x14, x3
        adc   x14, xzr, xzr

        adds  x11, x11, x7
        umulh x2,  x15, x9
        adds  x11, x11, x2
        umulh x3,  x16, x9
        adcs  x12, x12, x3
        umulh x2,  x17, x9
        adcs  x13, x13, x2
        umulh x3,  x8,  x9
        adcs  x14, x14, x3
        adc   x7,  xzr, xzr

        // np0 = Fr_np * product3[0];
        mul   x9, x4, x11

        // product3 = product3 + Fr_rawq * np0
        mul   x2,  x15, x9
        adds  x10, x11, x2
        mul   x3,  x16, x9
        adcs  x11, x12, x3
        mul   x2,  x17, x9
        adcs  x12, x13, x2
        mul   x3,  x8,  x9
        adcs  x13, x14, x3
        adc   x14, xzr, xzr

        adds  x11, x11, x7
        umulh x2,  x15, x9
        adds  x11, x11, x2
        umulh x3,  x16, x9
        adcs  x12, x12, x3
        umulh x2,  x17, x9
        adcs  x13, x13, x2
        umulh x3,  x8,  x9
        adcs  x14, x14, x3

        // result >= Fr_rawq
        subs x5, x11, x15
        sbcs x6, x12, x16
        sbcs x7, x13, x17
        sbcs x8, x14, x8

        b.hs Fr_rawMul1_done_s

        stp x11, x12, [x0]
        stp x13, x14, [x0, 16]
        ret

        Fr_rawMul1_done_s:
        stp x5, x6, [x0]
        stp x7, x8, [x0, 16]
        ret


// void Fr_rawFromMontgomery(FrRawElement pRawResult, FrRawElement pRawA)
Fr_rawFromMontgomery:
_Fr_rawFromMontgomery:
        ldp x10, x11, [x1]    //pRawA
        ldp x12, x13, [x1, 16]
        mov x14, xzr

        adr x4, Fr_np
        ldr x4, [x4]

        adr x3, Fr_rawq
        ldp x15, x16, [x3]
        ldp x17, x8,  [x3, 16]

        // np0 = Fr_np * product0[0];
        mul   x9, x4, x10

        // product0 = product0 + Fr_rawq * np0
        mul   x2,  x15, x9
        adds  x10, x10, x2
        mul   x3,  x16, x9
        adcs  x11, x11, x3
        mul   x2,  x17, x9
        adcs  x12, x12, x2
        mul   x3,  x8,  x9
        adcs  x13, x13, x3
        adc   x14, x14, xzr

        umulh x2,  x15, x9
        adds  x11, x11, x2
        umulh x3,  x16, x9
        adcs  x12, x12, x3
        umulh x2,  x17, x9
        adcs  x13, x13, x2
        umulh x3,  x8,  x9
        adcs  x14, x14, x3
        adc   x7,  xzr, xzr

        // np0 = Fr_np * product1[0];
        mul   x9, x4, x11

        // product1 = product1 + Fr_rawq * np0
        mul   x2,  x15, x9
        adds  x10, x11, x2
        mul   x3,  x16, x9
        adcs  x11, x12, x3
        mul   x2,  x17, x9
        adcs  x12, x13, x2
        mul   x3,  x8 , x9
        adcs  x13, x14, x3
        adc   x14, xzr, xzr

        adds x11,  x11, x7
        umulh x2,  x15, x9
        adcs  x11, x11, x2
        umulh x3,  x16, x9
        adcs  x12, x12, x3
        umulh x2,  x17, x9
        adcs  x13, x13, x2
        umulh x3,  x8 , x9
        adcs  x14, x14, x3
        adc   x7,  xzr, xzr

        // np0 = Fr_np * product2[0];
        mul   x9, x4, x11

        // product2 = product2 + Fr_rawq * np0
        mul   x2,  x15, x9
        adds  x10, x11, x2
        mul   x3,  x16, x9
        adcs  x11, x12, x3
        mul   x2,  x17, x9
        adcs  x12, x13, x2
        mul   x3,  x8,  x9
        adcs  x13, x14, x3
        adc   x14, xzr, xzr

        adds  x11, x11, x7
        umulh x2,  x15, x9
        adds  x11, x11, x2
        umulh x3,  x16, x9
        adcs  x12, x12, x3
        umulh x2,  x17, x9
        adcs  x13, x13, x2
        umulh x3,  x8,  x9
        adcs  x14, x14, x3
        adc   x7,  xzr, xzr

        // np0 = Fr_np * product3[0];
        mul   x9, x4, x11

        // product3 = product3 + Fr_rawq * np0
        mul   x2,  x15, x9
        adds  x10, x11, x2
        mul   x3,  x16, x9
        adcs  x11, x12, x3
        mul   x2,  x17, x9
        adcs  x12, x13, x2
        mul   x3,  x8,  x9
        adcs  x13, x14, x3
        adc   x14, xzr, xzr

        adds  x11, x11, x7
        umulh x2,  x15, x9
        adds  x11, x11, x2
        umulh x3,  x16, x9
        adcs  x12, x12, x3
        umulh x2,  x17, x9
        adcs  x13, x13, x2
        umulh x3,  x8,  x9
        adcs  x14, x14, x3

        // result >= Fr_rawq
        subs x5, x11, x15
        sbcs x6, x12, x16
        sbcs x7, x13, x17
        sbcs x8, x14, x8

        b.hs Fr_rawFromMontgomery_s

        stp x11, x12, [x0]
        stp x13, x14, [x0, 16]
        ret

Fr_rawFromMontgomery_s:
        stp x5, x6, [x0]
        stp x7, x8, [x0, 16]
        ret



// void Fr_rawCopy(FrRawElement pRawResult, FrRawElement pRawA)
Fr_rawCopy:
_Fr_rawCopy:
        ldp x2, x3, [x1]
        stp x2, x3, [x0]

        ldp x4, x5, [x1, 16]
        stp x4, x5, [x0, 16]
        ret


// void Fr_rawSwap(FrRawElement pRawResult, FrRawElement pRawA)
Fr_rawSwap:
_Fr_rawSwap:
        ldp  x3, x4, [x0]
        ldp  x7, x8, [x1]

        stp  x3, x4, [x1]
        stp  x7, x8, [x0]

        ldp  x5, x6,  [x0, 16]
        ldp  x9, x10, [x1, 16]

        stp  x5, x6,  [x1, 16]
        stp  x9, x10, [x0, 16]
        ret


// int Fr_rawIsEq(FrRawElement pRawA, FrRawElement pRawB)
Fr_rawIsEq:
_Fr_rawIsEq:
        ldp  x3, x4, [x0]
        ldp  x7, x8, [x1]
        eor x11, x3, x7
        eor x12, x4, x8

        ldp  x5, x6,  [x0, 16]
        ldp  x9, x10, [x1, 16]
        eor x13, x5, x9
        eor x14, x6, x10

        orr x15, x11, x12
        orr x16, x13, x14

        orr x0, x15, x16
        cmp  x0, xzr
        cset x0, eq
        ret


// int Fr_rawIsZero(FrRawElement rawA)
Fr_rawIsZero:
_Fr_rawIsZero:
        ldp x1, x2, [x0]
        orr x5, x1, x2

        ldp x3, x4, [x0, 16]
        orr x6, x3, x4

        orr  x0, x5, x6
        cmp  x0, xzr
        cset x0, eq
        ret


// void Fr_rawCopyS2L(FrRawElement pRawResult, int64_t val)
Fr_rawCopyS2L:
_Fr_rawCopyS2L:
        cmp  x1, xzr
        b.lt Fr_rawCopyS2L_adjust_neg

        stp x1,  xzr, [x0]
        stp xzr, xzr, [x0, 16]
        ret

Fr_rawCopyS2L_adjust_neg:
        adr x3, Fr_rawq
        ldp x5, x6, [x3]
        ldp x7, x8, [x3, 16]

        mov x9, -1

        adds x1, x1, x5
        adcs x2, x9, x6
        adcs x3, x9, x7
        adc  x4, x9, x8

        stp x1, x2, [x0]
        stp x3, x4, [x0, 16]
        ret


//int Fr_rawCmp(FrRawElement pRawA, FrRawElement pRawB)
Fr_rawCmp:
_Fr_rawCmp:
        ldp  x3, x4,  [x0]
        ldp  x5, x6,  [x0, 16]
        ldp  x7, x8,  [x1]
        ldp  x9, x10, [x1, 16]

        subs x3, x3, x7
        cset x0, ne

        sbcs x4, x4, x8
        cinc x0, x0, ne

        sbcs x5, x5, x9
        cinc x0, x0, ne

        sbcs x6, x6, x10
        cinc x0, x0, ne

        cneg x0, x0, lo
        ret

//void Fr_rawAnd(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB)
Fr_rawAnd:
_Fr_rawAnd:
        ldp x3, x4, [x1]
        ldp x7, x8, [x2]
        and x3, x3, x7
        and x4, x4, x8

        ldp x5, x6,  [x1, 16]
        ldp x9, x10, [x2, 16]
        and x5, x5, x9
        and x6, x6, x10

        and x6, x6, 0x3fffffffffffffff // lboMask

        adr x11, Fr_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        subs x7,  x3, x12
        sbcs x8,  x4, x13
        sbcs x9,  x5, x14
        sbcs x10, x6, x15

        csel x3, x7,  x3, hs
        csel x4, x8,  x4, hs
        csel x5, x9,  x5, hs
        csel x6, x10, x6, hs

        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret

//void Fr_rawOr(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB)
Fr_rawOr:
_Fr_rawOr:
        ldp x3, x4, [x1]
        ldp x7, x8, [x2]
        orr x3, x3, x7
        orr x4, x4, x8

        ldp x5, x6,  [x1, 16]
        ldp x9, x10, [x2, 16]
        orr x5, x5, x9
        orr x6, x6, x10

        and x6, x6, 0x3fffffffffffffff // lboMask

        adr x11, Fr_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        subs x7,  x3, x12
        sbcs x8,  x4, x13
        sbcs x9,  x5, x14
        sbcs x10, x6, x15

        csel x3, x7,  x3, hs
        csel x4, x8,  x4, hs
        csel x5, x9,  x5, hs
        csel x6, x10, x6, hs

        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret

//void Fr_rawXor(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB)
Fr_rawXor:
_Fr_rawXor:
        ldp x3, x4, [x1]
        ldp x7, x8, [x2]
        eor x3, x3, x7
        eor x4, x4, x8

        ldp x5, x6,  [x1, 16]
        ldp x9, x10, [x2, 16]
        eor x5, x5, x9
        eor x6, x6, x10

        and x6, x6, 0x3fffffffffffffff // lboMask

        adr x11, Fr_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        subs x7,  x3, x12
        sbcs x8,  x4, x13
        sbcs x9,  x5, x14
        sbcs x10, x6, x15

        csel x3, x7,  x3, hs
        csel x4, x8,  x4, hs
        csel x5, x9,  x5, hs
        csel x6, x10, x6, hs

        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret

//void Fr_rawShl(FrRawElement r, FrRawElement a, uint64_t b)
Fr_rawShl:
_Fr_rawShl:
        ldp x3, x4, [x1]
        ldp x5, x6, [x1, 16]

        ands x7, x2, 0x3f    // bit_shift = b % 64
        mov  x8, 0x3f
        mov  x9, 0x1
        sub  x8, x8, x7      // bit_shift augmenter to 64

        tbnz x2, 7, Fr_rawShl_word_shift_2
        tbnz x2, 6, Fr_rawShl_word_shift_1

Fr_rawShl_word_shift_0:
        lsl x13, x6,  x7
        lsr x15, x5,  x8
        lsr x15, x15, x9
        orr x13, x13, x15

        lsl x12, x5,  x7
        lsr x16, x4,  x8
        lsr x16, x16, x9
        orr x12, x12, x16

        lsl x11, x4,  x7
        lsr x17, x3,  x8
        lsr x17, x17, x9
        orr x11, x11, x17

        lsl x10, x3,  x7

        b Fr_rawShl_sub

Fr_rawShl_word_shift_1:
        lsl x13, x5,  x7
        lsr x15, x4,  x8
        lsr x15, x15, x9
        orr x13, x13, x15

        lsl x12, x4,  x7
        lsr x16, x3,  x8
        lsr x16, x16, x9
        orr x12, x12, x16

        lsl x11, x3,  x7
        mov x10, xzr

        b Fr_rawShl_sub

Fr_rawShl_word_shift_2:
        tbnz x2, 6, Fr_rawShl_word_shift_3

        lsl x13, x4,  x7
        lsr x15, x3,  x8
        lsr x15, x15, x9
        orr x13, x13, x15

        lsl x12, x3,  x7
        mov x11, xzr
        mov x10, xzr

        b Fr_rawShl_sub

Fr_rawShl_word_shift_3:
        lsl x13, x3, x7
        mov x12, xzr
        mov x11, xzr
        mov x10, xzr

Fr_rawShl_sub:
        and x13, x13, 0x3fffffffffffffff // lboMask

        adr x9, Fr_rawq
        ldp x14, x15, [x9]
        ldp x16, x17, [x9, 16]

        subs x3, x10, x14
        sbcs x4, x11, x15
        sbcs x5, x12, x16
        sbcs x6, x13, x17

        csel x10, x3, x10, hs
        csel x11, x4, x11, hs
        csel x12, x5, x12, hs
        csel x13, x6, x13, hs

        stp x10, x11, [x0]
        stp x12, x13, [x0, 16]
        ret


//void Fr_rawShr(FrRawElement r, FrRawElement a, uint64_t b)
Fr_rawShr:
_Fr_rawShr:
        ldp x3, x4, [x1]
        ldp x5, x6, [x1, 16]

        and x7, x2, 0x3f    // bit_shift = b % 64
        mov x8, 0x40
        sub x8, x8, x7      // bit_shift augmenter to 64

        tbnz x2, 7, Fr_rawShr_word_shift_2
        tbnz x2, 6, Fr_rawShr_word_shift_1

Fr_rawShr_word_shift_0:
        cbz x7, Fr_rawShr_word_shift_0_end

        lsr x3,  x3,  x7
        lsl x15, x4,  x8
        orr x3,  x3, x15

        lsr x4,  x4,  x7
        lsl x16, x5,  x8
        orr x4,  x4, x16

        lsr x5,  x5,  x7
        lsl x17, x6,  x8
        orr x5,  x5, x17

        lsr x6, x6,  x7

Fr_rawShr_word_shift_0_end:
        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret

Fr_rawShr_word_shift_1:
        cbz x7, Fr_rawShr_word_shift_1_end

        lsr x4,  x4,  x7
        lsl x15, x5,  x8
        orr x4,  x4, x15

        lsr x5,  x5,  x7
        lsl x16, x6,  x8
        orr x5,  x5, x16

        lsr x6, x6,  x7

Fr_rawShr_word_shift_1_end:
        stp x4, x5,  [x0]
        stp x6, xzr, [x0, 16]
        ret

Fr_rawShr_word_shift_2:
        tbnz x2, 6, Fr_rawShr_word_shift_3

        cbz x7, Fr_rawShr_word_shift_2_end

        lsr x5,  x5,  x7
        lsl x15, x6,  x8
        orr x5,  x5, x15

        lsr x6, x6,  x7

Fr_rawShr_word_shift_2_end:
        stp x5, x6, [x0]
        stp xzr, xzr, [x0, 16]
        ret

Fr_rawShr_word_shift_3:
        lsr x6, x6, x7

        stp x6,  xzr, [x0]
        stp xzr, xzr, [x0, 16]
        ret

//void Fr_rawNot(FrRawElement pRawResult, FrRawElement pRawA)
Fr_rawNot:
_Fr_rawNot:
        ldp x3, x4, [x1]
        mvn x3, x3
        mvn x4, x4

        ldp x5, x6,  [x1, 16]
        mvn x5, x5
        mvn x6, x6

        and x6, x6, 0x3fffffffffffffff // lboMask

        adr x11, Fr_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        subs x7,  x3, x12
        sbcs x8,  x4, x13
        sbcs x9,  x5, x14
        sbcs x10, x6, x15

        csel x3, x7,  x3, hs
        csel x4, x8,  x4, hs
        csel x5, x9,  x5, hs
        csel x6, x10, x6, hs

        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret


        .align 8
Fr_rawq:    .quad 0x43e1f593f0000001,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029
Fr_np:      .quad 0xc2e1f593efffffff
