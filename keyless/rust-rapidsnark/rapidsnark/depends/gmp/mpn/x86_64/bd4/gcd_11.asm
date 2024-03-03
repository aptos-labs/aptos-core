dnl  AMD64 mpn_gcd_11 optimised for AMD BD4, ZN1.

dnl  Based on the K7 gcd_1.asm, by Kevin Ryde.  Rehacked for AMD64 by Torbjorn
dnl  Granlund.

dnl  Copyright 2000-2002, 2005, 2009, 2011, 2012, 2017, 2019 Free Software
dnl  Foundation, Inc.

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


C	     cycles/bit (approx)
C AMD K8,K9	 -
C AMD K10	 -
C AMD bd1	 -
C AMD bd2	 -
C AMD bd3	 -
C AMD bd4	 3.73
C AMD bt1	 -
C AMD bt2	 -
C AMD zn1	 3.33
C AMD zn2	 3.48
C Intel P4	 -
C Intel CNR	 -
C Intel PNR	 -
C Intel NHM	 -
C Intel WSM	 -
C Intel SBR	 -
C Intel IBR	 -
C Intel HWL	 ?
C Intel BWL	 ?
C Intel SKL	 ?
C Intel atom	 -
C Intel SLM	 -
C Intel GLM	 -
C Intel GLM+	 -
C VIA nano	 -

define(`u0',    `%rdi')
define(`v0',    `%rsi')

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(64)
PROLOGUE(mpn_gcd_11)
	FUNC_ENTRY(2)
	mov	u0, %rax
	mov	v0, %rdx
	sub	u0, %rdx		C v - u
	jz	L(end)

	ALIGN(16)
L(top):	rep;bsf	%rdx, %rcx		C tzcnt!
	sub	v0, u0			C u - v
	cmovc	%rdx, u0		C u = |u - v|
	cmovc	%rax, v0		C v = min(u,v)
	shrx(	%rcx, u0, %rax)
	shrx(	%rcx, u0, u0)
	mov	v0, %rdx
	sub	%rax, %rdx		C v - u
	jnz	L(top)

L(end):	C rax = result
	C rdx = 0 for the benefit of internal gcd_22 call
	FUNC_EXIT()
	ret
EPILOGUE()
