dnl  AMD64 mpn_gcd_22.  Assumes useless bsf, useless shrd, useful tzcnt, shlx.

dnl  Copyright 2019 Free Software Foundation, Inc.

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


C	     cycles/bit
C AMD K8,K9	 -
C AMD K10	 -
C AMD bd1	 -
C AMD bd2	 -
C AMD bd3	 -
C AMD bd4	 6.7
C AMD bt1	 -
C AMD bt2	 -
C AMD zn1	 5.4
C AMD zn2	 5.5
C Intel P4	 -
C Intel CNR	 -
C Intel PNR	 -
C Intel NHM	 -
C Intel WSM	 -
C Intel SBR	 -
C Intel IBR	 -
C Intel HWL	 7.1
C Intel BWL	 5.5
C Intel SKL	 5.6
C Intel atom	 -
C Intel SLM	 -
C Intel GLM	 -
C Intel GLM+	 -
C VIA nano	 -


define(`u1',    `%rdi')
define(`u0',    `%rsi')
define(`v1',    `%rdx')
define(`v0',    `%rcx')

define(`s0',    `%r8')
define(`s1',    `%r9')
define(`t0',    `%r10')
define(`t1',    `%r11')
define(`cnt',   `%rax')

dnl ABI_SUPPORT(DOS64)	C returns mp_double_limb_t in memory
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(64)
PROLOGUE(mpn_gcd_22)
	FUNC_ENTRY(4)

	ALIGN(16)
L(top):	mov	v0, t0
	sub	u0, t0
	jz	L(lowz)		C	jump when low limb result = 0
	mov	v1, t1
	sbb	u1, t1

	rep;bsf	t0, cnt		C tzcnt!

	mov	u0, s0
	sub	v0, u0
	mov	u1, s1
	sbb	v1, u1

L(bck):	cmovc	t0, u0		C u = |u - v|
	cmovc	t1, u1		C u = |u - v|
	cmovc	s0, v0		C v = min(u,v)
	cmovc	s1, v1		C v = min(u,v)

	xor	R32(t0), R32(t0)
	sub	cnt, t0
	shlx(	t0, u1, s1)
	shrx(	cnt, u0, u0)
	shrx(	cnt, u1, u1)
	or	s1, u0

	test	v1, v1
	jnz	L(top)
	test	u1, u1
	jnz	L(top)

L(gcd_11):
	mov	v0, %rdi
C	mov	u0, %rsi
	TCALL(	mpn_gcd_11)

L(lowz):C We come here when v0 - u0 = 0
	C 1. If v1 - u1 = 0, then gcd is u = v.
	C 2. Else compute gcd_21({v1,v0}, |u1-v1|)
	mov	v1, t0
	sub	u1, t0
	je	L(end)

	xor	t1, t1
	mov	u0, s0
	mov	u1, s1
	rep;bsf	t0, cnt		C tzcnt!
	mov	u1, u0
	xor	u1, u1
	sub	v1, u0
	jmp	L(bck)

L(end):	mov	v0, %rax
	C mov	v1, %rdx
L(ret):	FUNC_EXIT()
	ret
EPILOGUE()
