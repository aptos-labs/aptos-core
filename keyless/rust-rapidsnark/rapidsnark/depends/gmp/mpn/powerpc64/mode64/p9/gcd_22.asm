dnl  PowerPC-64 mpn_gcd_22 optimised for POWER9.

dnl  Copyright 2000-2002, 2005, 2009, 2011-2013, 2019 Free Software Foundation,
dnl  Inc.

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

C		    cycles/bit (approx)
C POWER3/PPC630		 -
C POWER4/PPC970		 -
C POWER5		 -
C POWER6		 -
C POWER7		 -
C POWER8		 -
C POWER9		 9.58

C We define SLOW if this target uses a slow struct return mechanism, with
C r3 as an implicit parameter for the struct pointer.
undefine(`SLOW')dnl
ifdef(`AIX',`define(`SLOW',`due to AIX')',`
  ifdef(`DARWIN',,`
    ifdef(`ELFv2_ABI',,`define(`SLOW',`due to ELFv1')')dnl
  ')
')

ifdef(`SLOW',`
define(`IFSLOW', `$1')
define(`u1',    `r4')
define(`u0',    `r5')
define(`v1',    `r6')
define(`v0',    `r7')
',`
define(`IFSLOW', `')
define(`u1',    `r3')
define(`u0',    `r4')
define(`v1',    `r5')
define(`v0',    `r6')
')

define(`tmp',   `r0')
define(`t0',    `r8')
define(`t1',    `r9')
define(`s0',    `r10')
define(`s1',    `r11')
define(`cnt',   `r12')

ASM_START()
PROLOGUE(mpn_gcd_22)
	cmpld	cr7, v0, u0
L(top):	subfc	t0, v0, u0		C 0 12
	beq	cr7, L(lowz)
	subfe	t1, v1, u1		C 2 14
	subfe.	tmp, tmp, tmp		C 4	set cr0 from the carry bit
	subfc	s0, u0, v0		C 0
	subfe	s1, u1, v1		C 2

L(bck):	cnttzd	cnt, t0			C 2
	subfic	tmp, cnt, 64		C 4

	isel	v0, v0, u0, 2		C 6	use condition set by subfe
	isel	u0, t0, s0, 2		C 6
	isel	v1, v1, u1, 2		C 6
	isel	u1, t1, s1, 2		C 6

	srd	u0, u0, cnt		C 8
	sld	tmp, u1, tmp		C 8
	srd	u1, u1, cnt		C 8
	or	u0, u0, tmp		C 10

	or.	r0, u1, v1		C 10
	cmpld	cr7, v0, u0
	bne	L(top)


	b	L(odd)
	ALIGN(16)
L(top1):isel	v0, u0, v0, 29		C v = min(u,v)
	isel	u0, r10, r11, 29	C u = |u - v|
	srd	u0, u0, cnt
L(odd):	subf	r10, u0, v0		C r10 = v - u
	subf	r11, v0, u0		C r11 = u - v
	cmpld	cr7, v0, u0
	cnttzd	cnt, r10
	bne	cr7, L(top1)

ifdef(`SLOW',`
	std	v0, 0(r3)
	std	r10, 8(r3)
',`
	mr	r3, v0
	li	r4, 0
')
	blr


L(lowz):C We come here when v0 - u0 = 0
	C 1. If v1 - u1 = 0, then gcd is u = v.
	C 2. Else compute gcd_21({v1,v0}, |u1-v1|)
	subfc.	t0, v1, u1		C 2 8
	beq	L(end)
	li	t1, 0
	subfe.	tmp, tmp, tmp		C 4	set cr0 from the carry bit
	subf	s0, u1, v1		C 2
	li	s1, 0
	b	L(bck)

L(end):
ifdef(`SLOW',`
	std	v0, 0(r3)
	std	v1, 8(r3)
	blr
',`
	mr	r3, v0
	mr	r4, v1
	blr
')
EPILOGUE()
