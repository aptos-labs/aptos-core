dnl  ARM64 mpn_addmul_1 and mpn_submul_1

dnl  Contributed to the GNU project by Torbjörn Granlund.

dnl  Copyright 2013, 2015, 2017 Free Software Foundation, Inc.

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
C Cortex-A53	9.3-9.8
C Cortex-A57	 7.0
C X-Gene	 5.0

C NOTES
C  * It is possible to keep the carry chain alive between the addition blocks
C    and thus avoid csinc, but only for addmul_1.  Since that saves no time
C    on the tested pipelines, we keep addmul_1 and submul_1 similar.
C  * We could separate feed-in into 4 blocks, one for each residue (mod 4).
C    That is likely to save a few cycles.

changecom(blah)

define(`rp', `x0')
define(`up', `x1')
define(`n',  `x2')
define(`v0', `x3')

ifdef(`OPERATION_addmul_1', `
  define(`ADDSUB',	adds)
  define(`ADDSUBC',	adcs)
  define(`COND',	`cc')
  define(`func',	mpn_addmul_1)')
ifdef(`OPERATION_submul_1', `
  define(`ADDSUB',	subs)
  define(`ADDSUBC',	sbcs)
  define(`COND',	`cs')
  define(`func',	mpn_submul_1)')

MULFUNC_PROLOGUE(mpn_addmul_1 mpn_submul_1)

PROLOGUE(func)
	adds	x15, xzr, xzr

	tbz	n, #0, L(1)

	ldr	x4, [up],#8
	mul	x8, x4, v0
	umulh	x12, x4, v0
	ldr	x4, [rp]
	ADDSUB	x8, x4, x8
	csinc	x15, x12, x12, COND
	str	x8, [rp],#8

L(1):	tbz	n, #1, L(2)

	ldp	x4, x5, [up],#16
	mul	x8, x4, v0
	umulh	x12, x4, v0
	mul	x9, x5, v0
	umulh	x13, x5, v0
	adds	x8, x8, x15
	adcs	x9, x9, x12
	ldp	x4, x5, [rp]
	adc	x15, x13, xzr
	ADDSUB	x8, x4, x8
	ADDSUBC	x9, x5, x9
	csinc	x15, x15, x15, COND
	stp	x8, x9, [rp],#16

L(2):	lsr	n, n, #2
	cbz	n, L(le3)
	ldp	x4, x5, [up],#32
	ldp	x6, x7, [up,#-16]
	b	L(mid)
L(le3):	mov	x0, x15
	ret

	ALIGN(16)
L(top):	ldp	x4, x5, [up],#32
	ldp	x6, x7, [up,#-16]
	ADDSUB	x8, x16, x8
	ADDSUBC	x9, x17, x9
	stp	x8, x9, [rp],#32
	ADDSUBC	x10, x12, x10
	ADDSUBC	x11, x13, x11
	stp	x10, x11, [rp,#-16]
	csinc	x15, x15, x15, COND
L(mid):	sub	n, n, #1
	mul	x8, x4, v0
	umulh	x12, x4, v0
	mul	x9, x5, v0
	umulh	x13, x5, v0
	adds	x8, x8, x15
	mul	x10, x6, v0
	umulh	x14, x6, v0
	adcs	x9, x9, x12
	mul	x11, x7, v0
	umulh	x15, x7, v0
	adcs	x10, x10, x13
	ldp	x16, x17, [rp]
	adcs	x11, x11, x14
	ldp	x12, x13, [rp,#16]
	adc	x15, x15, xzr
	cbnz	n, L(top)

	ADDSUB	x8, x16, x8
	ADDSUBC	x9, x17, x9
	ADDSUBC	x10, x12, x10
	ADDSUBC	x11, x13, x11
	stp	x8, x9, [rp]
	stp	x10, x11, [rp,#16]
	csinc	x0, x15, x15, COND
	ret
EPILOGUE()
