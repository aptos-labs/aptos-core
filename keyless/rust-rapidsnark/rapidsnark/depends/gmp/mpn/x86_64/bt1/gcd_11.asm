dnl  AMD64 mpn_gcd_11 -- 1 x 1 gcd.

dnl  Based on the K7 gcd_1.asm, by Kevin Ryde.  Rehacked for AMD64 by Torbjorn
dnl  Granlund.

dnl  Copyright 2000-2002, 2005, 2009, 2011, 2012, 2017 Free Software
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


C	     cycles/bit
C AMD K8,K9	 ?
C AMD K10	 ?
C AMD bd1	 ?
C AMD bd2	 ?
C AMD bd3	 ?
C AMD bd4	 ?
C AMD bt1	 5.4
C AMD bt2	 ?
C AMD zn1	 ?
C AMD zn2	 ?
C Intel P4	 ?
C Intel CNR	 ?
C Intel PNR	 ?
C Intel NHM	 ?
C Intel WSM	 ?
C Intel SBR	 ?
C Intel IBR	 ?
C Intel HWL	 ?
C Intel BWL	 ?
C Intel SKL	 ?
C Intel atom	 ?
C Intel SLM	 ?
C Intel GLM	 ?
C Intel GLM+	 ?
C VIA nano	 ?


C ctz_table[n] is the number of trailing zeros on n, or MAXSHIFT if n==0.

deflit(MAXSHIFT, 8)
deflit(MASK, eval((m4_lshift(1,MAXSHIFT))-1))

DEF_OBJECT(ctz_table,64)
	.byte	MAXSHIFT
forloop(i,1,MASK,
`	.byte	m4_count_trailing_zeros(i)
')
END_OBJECT(ctz_table)

define(`u0',    `%rdi')
define(`v0',    `%rsi')

define(`cnt',   `%rcx')
define(`s0',    `%rax')
define(`t0',    `%rdx')

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(64)
PROLOGUE(mpn_gcd_11)
	FUNC_ENTRY(2)
	LEA(	ctz_table, %r10)
	mov	v0, t0
	sub	u0, t0
	jz	L(end)

	ALIGN(16)
L(top):	mov	u0, s0
	sub	v0, u0
	cmovc	t0, u0		C u = |u - v|
	cmovc	s0, v0		C v = min(u,v)
	and	$MASK, R32(t0)
	movzbl	(%r10,t0), R32(cnt)
	jz	L(count_better)
L(shr):	shr	R8(cnt), u0
	mov	v0, t0
	sub	u0, t0
	jnz	L(top)

L(end):	mov	v0, %rax
	C rdx = 0 for the benefit of internal gcd_22 call
	FUNC_EXIT()
	ret

L(count_better):
	bsf	u0, cnt
	jmp	L(shr)
EPILOGUE()
