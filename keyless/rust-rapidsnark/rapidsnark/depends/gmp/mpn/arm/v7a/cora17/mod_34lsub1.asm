dnl  ARM mpn_mod_34lsub1 -- remainder modulo 2^24-1.

dnl  Copyright 2012, 2013, 2018 Free Software Foundation, Inc.

dnl  This file is part of the GNU MP Library.
dnl
dnl  The GNU MP Library is free software; you can redistribute it and/or modify
dnl  it under the terms of either:
dnl
dnl    * the GNU Lesser General Public License as published by the Free
dnl      Software Foundation; either version 3 of the License, or (at your
dnl      option) any later version.
dnl
dnl  or
dnl
dnl    * the GNU General Public License as published by the Free Software
dnl      Foundation; either version 2 of the License, or (at your option) any
dnl      later version.
dnl
dnl  or both in parallel, as here.
dnl
dnl  The GNU MP Library is distributed in the hope that it will be useful, but
dnl  WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
dnl  or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License
dnl  for more details.
dnl
dnl  You should have received copies of the GNU General Public License and the
dnl  GNU Lesser General Public License along with the GNU MP Library.  If not,
dnl  see https://www.gnu.org/licenses/.

include(`../config.m4')

C	     cycles/limb
C StrongARM	 ?
C XScale	 ?
C Cortex-A5	 2.67
C Cortex-A7	 2.37
C Cortex-A8	 2.34
C Cortex-A9	 ?
C Cortex-A15	 1.39
C Cortex-A17	 1.60
C Cortex-A53	 2.51

define(`ap',	r0)
define(`n',	r1)

C mp_limb_t mpn_mod_34lsub1 (mp_srcptr up, mp_size_t n)

C TODO
C  * Write cleverer summation code.
C  * Consider loading 6 64-bit aligned registers at a time, to approach 1 c/l.

ASM_START()
	TEXT
	ALIGN(32)
PROLOGUE(mpn_mod_34lsub1)
	push	{ r4, r5, r6, r7 }

	subs	n, n, #3
	mov	r7, #0
	blt	L(le2)			C n <= 2

	ldmia	ap!, { r2, r3, r12 }
	subs	n, n, #3
	blt	L(sum)			C n <= 5
	mov	r7, #0
	b	L(mid)

L(top):	adds	r2, r2, r4
	adcs	r3, r3, r5
	adcs	r12, r12, r6
	adc	r7, r7, #0
L(mid):	ldmia	ap!, { r4, r5, r6 }
	subs	n, n, #3
	bpl	L(top)

	adds	r2, r2, r4
	adcs	r3, r3, r5
	adcs	r12, r12, r6
	adc	r7, r7, #0		C r7 <= 1

L(sum):	cmn	n, #2
	movlo	r4, #0
	ldrhs	r4, [ap], #4
	movls	r5, #0
	ldrhi	r5, [ap], #4

	adds	r2, r2, r4
	adcs	r3, r3, r5
	adcs	r12, r12, #0
	adc	r7, r7, #0		C r7 <= 2

L(sum2):
	bic	r0, r2, #0xff000000
	add	r0, r0, r2, lsr #24
	add	r0, r0, r7

	mov	r7, r3, lsl #8
	bic	r2, r7, #0xff000000
	add	r0, r0, r2
	add	r0, r0, r3, lsr #16

	mov	r2, r12, lsl #16
	bic	r1, r2, #0xff000000
	add	r0, r0, r1
	add	r0, r0, r12, lsr #8

	pop	{ r4, r5, r6, r7 }
	return	lr

L(le2):	cmn	n, #1
	bne	L(1)
	ldmia	ap!, { r2, r3 }
	mov	r12, #0
	b	L(sum2)
L(1):	ldr	r2, [ap]
	bic	r0, r2, #0xff000000
	add	r0, r0, r2, lsr #24
	pop	{ r4, r5, r6, r7 }
	return	lr
EPILOGUE()
