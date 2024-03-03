    .global Fq_rawAdd
    .global Fq_rawAddLS
    .global Fq_rawSub
    .global Fq_rawSubRegular
    .global Fq_rawNeg
    .global Fq_rawNegLS
    .global Fq_rawSubSL
    .global Fq_rawSubLS
    .global Fq_rawMMul
    .global Fq_rawMMul1
    .global Fq_rawFromMontgomery
    .global Fq_rawCopy
    .global Fq_rawSwap
    .global Fq_rawIsEq
    .global Fq_rawIsZero
    .global Fq_rawCopyS2L
    .global Fq_rawCmp
    .global Fq_rawAnd
    .global Fq_rawOr
    .global Fq_rawXor
    .global Fq_rawShr
    .global Fq_rawShl
    .global Fq_rawNot

    .global _Fq_rawAdd
    .global _Fq_rawAddLS
    .global _Fq_rawSub
    .global _Fq_rawSubRegular
    .global _Fq_rawNeg
    .global _Fq_rawNegLS
    .global _Fq_rawSubSL
    .global _Fq_rawSubLS
    .global _Fq_rawMMul
    .global _Fq_rawMMul1
    .global _Fq_rawFromMontgomery
    .global _Fq_rawCopy
    .global _Fq_rawSwap
    .global _Fq_rawIsEq
    .global _Fq_rawIsZero
    .global _Fq_rawCopyS2L
    .global _Fq_rawCmp
    .global _Fq_rawAnd
    .global _Fq_rawOr
    .global _Fq_rawXor
    .global _Fq_rawShr
    .global _Fq_rawShl
    .global _Fq_rawNot

    .text
    .align 4

// void Fq_rawAdd(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB)
Fq_rawAdd:
_Fq_rawAdd:
        ldp  x3, x4, [x1]
        ldp  x7, x8, [x2]
        adds x3, x3, x7
        adcs x4, x4, x8

        ldp  x5, x6,  [x1, 16]
        ldp  x9, x10, [x2, 16]
        adcs x5, x5, x9
        adcs x6, x6, x10

        cset x16, cs

        adr x11, Fq_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        subs x7,  x3, x12
        sbcs x8,  x4, x13
        sbcs x9,  x5, x14
        sbcs x10, x6, x15

        cbnz x16, Fq_rawAdd_done_s
        b.hs      Fq_rawAdd_done_s

        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret

Fq_rawAdd_done_s:
        stp x7, x8,  [x0]
        stp x9, x10, [x0, 16]
        ret


//void Fq_rawAddLS(FqRawElement pRawResult, FqRawElement pRawA, uint64_t rawB)
Fq_rawAddLS:
_Fq_rawAddLS:
        ldp  x3, x4, [x1]
        adds x3, x3, x2
        adcs x4, x4, xzr

        ldp  x5, x6,  [x1, 16]
        adcs x5, x5, xzr
        adcs x6, x6, xzr

        cset x16, cs

        adr x11, Fq_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        subs x7,  x3, x12
        sbcs x8,  x4, x13
        sbcs x9,  x5, x14
        sbcs x10, x6, x15

        cbnz x16, Fq_rawAddLS_done_s
        b.hs      Fq_rawAddLS_done_s

        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret

Fq_rawAddLS_done_s:
        stp x7, x8,  [x0]
        stp x9, x10, [x0, 16]
        ret


// void Fq_rawSub(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB)
Fq_rawSub:
_Fq_rawSub:
        ldp  x3, x4, [x1]
        ldp  x7, x8, [x2]
        subs x3, x3, x7
        sbcs x4, x4, x8

        ldp  x5, x6,  [x1, 16]
        ldp  x9, x10, [x2, 16]
        sbcs x5, x5, x9
        sbcs x6, x6, x10

        b.cs Fq_rawSub_done

        adr x11, Fq_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        adds x3, x3, x12
        adcs x4, x4, x13
        adcs x5, x5, x14
        adc  x6, x6, x15

Fq_rawSub_done:
        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret


//void Fq_rawSubRegular(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB)
Fq_rawSubRegular:
_Fq_rawSubRegular:
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

//void Fq_rawSubSL(FqRawElement pRawResult, uint64_t rawA, FqRawElement pRawB)
Fq_rawSubSL:
_Fq_rawSubSL:
        ldp  x7, x8, [x2]
        subs x3, x1,  x7
        sbcs x4, xzr, x8

        ldp  x9, x10, [x2, 16]
        sbcs x5, xzr, x9
        sbcs x6, xzr, x10

        b.cs Fq_rawSubSL_done

        adr x11, Fq_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        adds x3, x3, x12
        adcs x4, x4, x13
        adcs x5, x5, x14
        adc  x6, x6, x15

Fq_rawSubSL_done:
        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret


//void Fq_rawSubLS(FqRawElement pRawResult, FqRawElement pRawA, uint64_t rawB)
Fq_rawSubLS:
_Fq_rawSubLS:
        ldp  x3, x4, [x1]
        subs x3, x3, x2
        sbcs x4, x4, xzr

        ldp  x5, x6,  [x1, 16]
        sbcs x5, x5, xzr
        sbcs x6, x6, xzr

        b.cs Fq_rawSubLS_done

        adr x11, Fq_rawq
        ldp x12, x13, [x11]
        ldp x14, x15, [x11, 16]

        adds x3, x3, x12
        adcs x4, x4, x13
        adcs x5, x5, x14
        adc  x6, x6, x15

Fq_rawSubLS_done:
        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret


// void Fq_rawNeg(FqRawElement pRawResult, FqRawElement pRawA)
Fq_rawNeg:
_Fq_rawNeg:
        ldp x2, x3, [x1]
        orr x6, x2, x3

        ldp x4, x5, [x1, 16]
        orr x7, x4, x5
        orr x8, x6, x7

        cbz x8, Fq_rawNeg_done_zero

        adr x10, Fq_rawq
        ldp x11, x12, [x10]
        ldp x13, x14, [x10, 16]

        subs x2, x11, x2
        sbcs x3, x12, x3
        sbcs x4, x13, x4
        sbc  x5, x14, x5

        stp x2, x3, [x0]
        stp x4, x5, [x0, 16]
        ret

Fq_rawNeg_done_zero:
        stp xzr, xzr, [x0]
        stp xzr, xzr, [x0, 16]
        ret


//void Fq_rawNegLS(FqRawElement pRawResult, FqRawElement pRawA, uint64_t rawB)
Fq_rawNegLS:
_Fq_rawNegLS:
        ldp x3, x4, [x1]
        ldp x5, x6, [x1, 16]

        adr x11, Fq_rawq
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

        cbz x17, Fq_rawNegLS_done

        adds x7,  x7,  x12
        adcs x8,  x8,  x13
        adcs x9,  x9,  x14
        adc  x10, x10, x15


Fq_rawNegLS_done:
        stp x7, x8,  [x0]
        stp x9, x10, [x0, 16]
        ret


// void Fq_rawMMul(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB)
Fq_rawMMul:
_Fq_rawMMul:
        ldr x3,     [x1]    //pRawA[0]
        ldp x5, x6, [x2]    //pRawB
        ldp x7, x8, [x2, 16]

        adr x4, Fq_np
        ldr x4, [x4]

        str x28, [sp, #-16]!

        adr x2, Fq_rawq
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

        // np0 = Fq_np * product0[0];
        mul   x9, x4, x10

        // product0 = product0 + Fq_rawq * np0
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

        // np0 = Fq_np * product1[0];
        mul   x9, x4, x10

        // product1 = product1 + Fq_rawq * np0
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

        // np0 = Fq_np * product2[0];
        mul   x9, x4, x10

        // product2 = product2 + Fq_rawq * np0
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

        // np0 = Fq_np * product3[0];
        mul   x9, x4, x10

        // product3 = product3 + Fq_rawq * np0
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

        // result >= Fq_rawq
        subs x5, x11, x15
        sbcs x6, x12, x16
        sbcs x7, x13, x17
        sbcs x8, x14, x28

        ldr x28, [sp], #16

        b.hs Fq_rawMul_done_s

        stp x11, x12, [x0]
        stp x13, x14, [x0, 16]
        ret

Fq_rawMul_done_s:
        stp x5, x6, [x0]
        stp x7, x8, [x0, 16]
        ret


// void Fq_rawMMul1(FqRawElement pRawResult, FqRawElement pRawA, uint64_t pRawB)
Fq_rawMMul1:
_Fq_rawMMul1:
        ldp x5, x6, [x1]    //pRawA
        ldp x7, x8, [x1, 16]

        adr x4, Fq_np
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

        adr x3, Fq_rawq
        ldp x15, x16, [x3]
        ldp x17, x8,  [x3, 16]

        // np0 = Fq_np * product0[0];
        mul   x9, x4, x10

        // product0 = product0 + Fq_rawq * np0
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

        // np0 = Fq_np * product1[0];
        mul   x9, x4, x11

        // product1 = product1 + Fq_rawq * np0
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

        // np0 = Fq_np * product2[0];
        mul   x9, x4, x11

        // product2 = product2 + Fq_rawq * np0
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

        // np0 = Fq_np * product3[0];
        mul   x9, x4, x11

        // product3 = product3 + Fq_rawq * np0
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

        // result >= Fq_rawq
        subs x5, x11, x15
        sbcs x6, x12, x16
        sbcs x7, x13, x17
        sbcs x8, x14, x8

        b.hs Fq_rawMul1_done_s

        stp x11, x12, [x0]
        stp x13, x14, [x0, 16]
        ret

        Fq_rawMul1_done_s:
        stp x5, x6, [x0]
        stp x7, x8, [x0, 16]
        ret


// void Fq_rawFromMontgomery(FqRawElement pRawResult, FqRawElement pRawA)
Fq_rawFromMontgomery:
_Fq_rawFromMontgomery:
        ldp x10, x11, [x1]    //pRawA
        ldp x12, x13, [x1, 16]
        mov x14, xzr

        adr x4, Fq_np
        ldr x4, [x4]

        adr x3, Fq_rawq
        ldp x15, x16, [x3]
        ldp x17, x8,  [x3, 16]

        // np0 = Fq_np * product0[0];
        mul   x9, x4, x10

        // product0 = product0 + Fq_rawq * np0
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

        // np0 = Fq_np * product1[0];
        mul   x9, x4, x11

        // product1 = product1 + Fq_rawq * np0
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

        // np0 = Fq_np * product2[0];
        mul   x9, x4, x11

        // product2 = product2 + Fq_rawq * np0
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

        // np0 = Fq_np * product3[0];
        mul   x9, x4, x11

        // product3 = product3 + Fq_rawq * np0
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

        // result >= Fq_rawq
        subs x5, x11, x15
        sbcs x6, x12, x16
        sbcs x7, x13, x17
        sbcs x8, x14, x8

        b.hs Fq_rawFromMontgomery_s

        stp x11, x12, [x0]
        stp x13, x14, [x0, 16]
        ret

Fq_rawFromMontgomery_s:
        stp x5, x6, [x0]
        stp x7, x8, [x0, 16]
        ret



// void Fq_rawCopy(FqRawElement pRawResult, FqRawElement pRawA)
Fq_rawCopy:
_Fq_rawCopy:
        ldp x2, x3, [x1]
        stp x2, x3, [x0]

        ldp x4, x5, [x1, 16]
        stp x4, x5, [x0, 16]
        ret


// void Fq_rawSwap(FqRawElement pRawResult, FqRawElement pRawA)
Fq_rawSwap:
_Fq_rawSwap:
        ldp  x3, x4, [x0]
        ldp  x7, x8, [x1]

        stp  x3, x4, [x1]
        stp  x7, x8, [x0]

        ldp  x5, x6,  [x0, 16]
        ldp  x9, x10, [x1, 16]

        stp  x5, x6,  [x1, 16]
        stp  x9, x10, [x0, 16]
        ret


// int Fq_rawIsEq(FqRawElement pRawA, FqRawElement pRawB)
Fq_rawIsEq:
_Fq_rawIsEq:
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


// int Fq_rawIsZero(FqRawElement rawA)
Fq_rawIsZero:
_Fq_rawIsZero:
        ldp x1, x2, [x0]
        orr x5, x1, x2

        ldp x3, x4, [x0, 16]
        orr x6, x3, x4

        orr  x0, x5, x6
        cmp  x0, xzr
        cset x0, eq
        ret


// void Fq_rawCopyS2L(FqRawElement pRawResult, int64_t val)
Fq_rawCopyS2L:
_Fq_rawCopyS2L:
        cmp  x1, xzr
        b.lt Fq_rawCopyS2L_adjust_neg

        stp x1,  xzr, [x0]
        stp xzr, xzr, [x0, 16]
        ret

Fq_rawCopyS2L_adjust_neg:
        adr x3, Fq_rawq
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


//int Fq_rawCmp(FqRawElement pRawA, FqRawElement pRawB)
Fq_rawCmp:
_Fq_rawCmp:
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

//void Fq_rawAnd(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB)
Fq_rawAnd:
_Fq_rawAnd:
        ldp x3, x4, [x1]
        ldp x7, x8, [x2]
        and x3, x3, x7
        and x4, x4, x8

        ldp x5, x6,  [x1, 16]
        ldp x9, x10, [x2, 16]
        and x5, x5, x9
        and x6, x6, x10

        and x6, x6, 0x3fffffffffffffff // lboMask

        adr x11, Fq_rawq
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

//void Fq_rawOr(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB)
Fq_rawOr:
_Fq_rawOr:
        ldp x3, x4, [x1]
        ldp x7, x8, [x2]
        orr x3, x3, x7
        orr x4, x4, x8

        ldp x5, x6,  [x1, 16]
        ldp x9, x10, [x2, 16]
        orr x5, x5, x9
        orr x6, x6, x10

        and x6, x6, 0x3fffffffffffffff // lboMask

        adr x11, Fq_rawq
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

//void Fq_rawXor(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB)
Fq_rawXor:
_Fq_rawXor:
        ldp x3, x4, [x1]
        ldp x7, x8, [x2]
        eor x3, x3, x7
        eor x4, x4, x8

        ldp x5, x6,  [x1, 16]
        ldp x9, x10, [x2, 16]
        eor x5, x5, x9
        eor x6, x6, x10

        and x6, x6, 0x3fffffffffffffff // lboMask

        adr x11, Fq_rawq
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

//void Fq_rawShl(FqRawElement r, FqRawElement a, uint64_t b)
Fq_rawShl:
_Fq_rawShl:
        ldp x3, x4, [x1]
        ldp x5, x6, [x1, 16]

        ands x7, x2, 0x3f    // bit_shift = b % 64
        mov  x8, 0x3f
        mov  x9, 0x1
        sub  x8, x8, x7      // bit_shift augmenter to 64

        tbnz x2, 7, Fq_rawShl_word_shift_2
        tbnz x2, 6, Fq_rawShl_word_shift_1

Fq_rawShl_word_shift_0:
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

        b Fq_rawShl_sub

Fq_rawShl_word_shift_1:
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

        b Fq_rawShl_sub

Fq_rawShl_word_shift_2:
        tbnz x2, 6, Fq_rawShl_word_shift_3

        lsl x13, x4,  x7
        lsr x15, x3,  x8
        lsr x15, x15, x9
        orr x13, x13, x15

        lsl x12, x3,  x7
        mov x11, xzr
        mov x10, xzr

        b Fq_rawShl_sub

Fq_rawShl_word_shift_3:
        lsl x13, x3, x7
        mov x12, xzr
        mov x11, xzr
        mov x10, xzr

Fq_rawShl_sub:
        and x13, x13, 0x3fffffffffffffff // lboMask

        adr x9, Fq_rawq
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


//void Fq_rawShr(FqRawElement r, FqRawElement a, uint64_t b)
Fq_rawShr:
_Fq_rawShr:
        ldp x3, x4, [x1]
        ldp x5, x6, [x1, 16]

        and x7, x2, 0x3f    // bit_shift = b % 64
        mov x8, 0x40
        sub x8, x8, x7      // bit_shift augmenter to 64

        tbnz x2, 7, Fq_rawShr_word_shift_2
        tbnz x2, 6, Fq_rawShr_word_shift_1

Fq_rawShr_word_shift_0:
        cbz x7, Fq_rawShr_word_shift_0_end

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

Fq_rawShr_word_shift_0_end:
        stp x3, x4, [x0]
        stp x5, x6, [x0, 16]
        ret

Fq_rawShr_word_shift_1:
        cbz x7, Fq_rawShr_word_shift_1_end

        lsr x4,  x4,  x7
        lsl x15, x5,  x8
        orr x4,  x4, x15

        lsr x5,  x5,  x7
        lsl x16, x6,  x8
        orr x5,  x5, x16

        lsr x6, x6,  x7

Fq_rawShr_word_shift_1_end:
        stp x4, x5,  [x0]
        stp x6, xzr, [x0, 16]
        ret

Fq_rawShr_word_shift_2:
        tbnz x2, 6, Fq_rawShr_word_shift_3

        cbz x7, Fq_rawShr_word_shift_2_end

        lsr x5,  x5,  x7
        lsl x15, x6,  x8
        orr x5,  x5, x15

        lsr x6, x6,  x7

Fq_rawShr_word_shift_2_end:
        stp x5, x6, [x0]
        stp xzr, xzr, [x0, 16]
        ret

Fq_rawShr_word_shift_3:
        lsr x6, x6, x7

        stp x6,  xzr, [x0]
        stp xzr, xzr, [x0, 16]
        ret

//void Fq_rawNot(FqRawElement pRawResult, FqRawElement pRawA)
Fq_rawNot:
_Fq_rawNot:
        ldp x3, x4, [x1]
        mvn x3, x3
        mvn x4, x4

        ldp x5, x6,  [x1, 16]
        mvn x5, x5
        mvn x6, x6

        and x6, x6, 0x3fffffffffffffff // lboMask

        adr x11, Fq_rawq
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
Fq_rawq:    .quad 0x3c208c16d87cfd47,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029
Fq_np:      .quad 0x87d20782e4866389
