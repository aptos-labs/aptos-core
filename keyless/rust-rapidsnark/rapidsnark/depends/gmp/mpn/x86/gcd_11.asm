dnl  x86 mpn_gcd_11 optimised for processors with slow BSF.

dnl  Based on C version.

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

dnl  Rudimentary code for x86-32, i.e. for CPUs without cmov.  Also, the bsf
dnl  instruction is assumed to be so slow it is useless.  Instead a teble is
dnl  used.
dnl
dnl  The loop benefits from OoO, in-order CPUs might want a different loop.
dnl  The ebx and ecx registers could be combined if the assigment of ecx were
dnl  postponed until ebx died, but that would at least hurt in-order CPUs.

C	     cycles/bit (approx)
C AMD K7	 ?
C AMD K8,K9	 ?
C AMD K10	 ?
C AMD bd1	 ?
C AMD bd2	 ?
C AMD bd3	 ?
C AMD bd4	 ?
C AMD bt1	 ?
C AMD bt2	 ?
C AMD zn1	 ?
C AMD zn2	 ?
C Intel P4-2	 ?
C Intel P4-3/4	 ?
C Intel P6/13	 ?
C Intel CNR	 ?
C Intel NHM	 ?
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
C Numbers measured with: speed -CD -s8-32 -t24 mpn_gcd_1

deflit(MAXSHIFT, 6)
deflit(MASK, eval((m4_lshift(1,MAXSHIFT))-1))

DEF_OBJECT(ctz_table,64)
	.byte	MAXSHIFT
forloop(i,1,MASK,
`	.byte	m4_count_trailing_zeros(i)
')
END_OBJECT(ctz_table)

define(`u0',    `%eax')
define(`v0',    `%edx')


ASM_START()
	TEXT
	ALIGN(16)
PROLOGUE(mpn_gcd_11)
	push	%edi
	push	%esi
	push	%ebx

	mov	16(%esp), u0
	mov	20(%esp), v0
	LEAL(	ctz_table, %esi)
	sub	v0, u0			C u = u - v		0
	jz	L(end)

	ALIGN(16)
L(top):	sbb	%ebx, %ebx		C mask			1
	mov	u0, %edi		C			1
	mov	u0, %ecx		C			1
	and	%ebx, %edi		C			2
	xor	%ebx, u0		C			2
	add	%edi, v0		C v = min(u.v)		3
	sub	%ebx, u0		C u = |u - v|		3
L(mid):	and	$MASK, %ecx		C			2
	movzbl	(%esi,%ecx), %ecx	C			3
	jz	L(shift_alot)
	shr	%cl, u0			C			4
	sub	v0, u0			C u = u - v		0,5
	jnz	L(top)

L(end):	mov	v0, %eax
	pop	%ebx
	pop	%esi
	pop	%edi
	ret

L(shift_alot):
	shr	$MAXSHIFT, u0
	mov	u0, %ecx
	jmp	L(mid)
EPILOGUE()
ASM_END()
