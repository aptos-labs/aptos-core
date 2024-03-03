dnl  ARM64 mpn_bdiv_q_1, mpn_pi1_bdiv_q_1 -- Hensel division by 1-limb divisor.

dnl  Contributed to the GNU project by Torbjörn Granlund.

dnl  Copyright 2012, 2017 Free Software Foundation, Inc.

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

C               cycles/limb
C               norm   unorm
C Cortex-A53	12	15
C Cortex-A57	12	12
C Cortex-A72
C Cortex-A73
C X-Gene	11	11

C TODO
C  * Scheduling of umulh later in the unorm loop brings A53 time to 12 c/l.
C    Unfortunately, that requires software pipelining.

define(`rp',  `x0')
define(`up',  `x1')
define(`n',   `x2')
define(`d',   `x3')
define(`di',  `x4')		C	just mpn_pi1_bdiv_q_1
define(`cnt', `x5')		C	just mpn_pi1_bdiv_q_1

define(`cy',  `r7')
define(`tnc', `x8')

ASM_START()
PROLOGUE(mpn_bdiv_q_1)

	rbit	x6, d
	clz	cnt, x6
	lsr	d, d, cnt

	LEA_HI(	x7, binvert_limb_table)
	ubfx	x6, d, 1, 7
	LEA_LO(	x7, binvert_limb_table)
	ldrb	w6, [x7, x6]
	ubfiz	x7, x6, 1, 8
	umull	x6, w6, w6
	msub	x6, x6, d, x7
	lsl	x7, x6, 1
	mul	x6, x6, x6
	msub	x6, x6, d, x7
	lsl	x7, x6, 1
	mul	x6, x6, x6
	msub	di, x6, d, x7

	b	GSYM_PREFIX`'mpn_pi1_bdiv_q_1
EPILOGUE()

PROLOGUE(mpn_pi1_bdiv_q_1)
	sub	n, n, #1
	subs	x6, x6, x6		C clear r6 and C flag
	ldr	x9, [up],#8
	cbz	cnt, L(norm)

L(unorm):
	lsr	x12, x9, cnt
	cbz	n, L(eu1)
	sub	tnc, xzr, cnt

L(tpu):	ldr	x9, [up],#8
	lsl	x7, x9, tnc
	orr	x7, x7, x12
	sbcs	x6, x7, x6
	mul	x7, x6, di
	str	x7, [rp],#8
	lsr	x12, x9, cnt
	umulh	x6, x7, d
	sub	n, n, #1
	cbnz	n, L(tpu)

L(eu1):	sbcs	x6, x12, x6
	mul	x6, x6, di
	str	x6, [rp]
	ret

L(norm):
	mul	x5, x9, di
	str	x5, [rp],#8
	cbz	n, L(en1)

L(tpn):	ldr	x9, [up],#8
	umulh	x5, x5, d
	sbcs	x5, x9, x5
	mul	x5, x5, di
	str	x5, [rp],#8
	sub	n, n, #1
	cbnz	n, L(tpn)

L(en1):	ret
EPILOGUE()
