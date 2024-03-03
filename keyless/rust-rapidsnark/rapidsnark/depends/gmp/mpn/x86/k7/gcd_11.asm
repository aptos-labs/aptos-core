dnl  x86 mpn_gcd_11 optimised for AMD K7.

dnl  Contributed to the GNU project by by Kevin Ryde.  Rehacked by Torbjorn
dnl  Granlund.

dnl  Copyright 2000-2002, 2005, 2009, 2011, 2012, 2014, 2015 Free Software
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
C AMD K7	 5.31
C AMD K8,K9	 5.33
C AMD K10	 5.30
C AMD bd1	 ?
C AMD bobcat	 7.02
C Intel P4-2	10.1
C Intel P4-3/4	10.0
C Intel P6/13	 5.88
C Intel core2	 6.26
C Intel NHM	 6.83
C Intel SBR	 8.50
C Intel atom	 8.90
C VIA nano	 ?
C Numbers measured with: speed -CD -s16-32 -t16 mpn_gcd_1


C ctz_table[n] is the number of trailing zeros on n, or MAXSHIFT if n==0.

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

	mov	12(%esp), %eax
	mov	16(%esp), %edx

	LEAL(	ctz_table, %esi)
	jmp	L(odd)

	ALIGN(16)			C
L(top):	cmovc(	%ecx, %eax)		C u = |v - u|
	cmovc(	%edi, %edx)		C v = min(u,v)
L(mid):	and	$MASK, %ecx		C
	movzbl	(%esi,%ecx), %ecx	C
	jz	L(shift_alot)		C
	shr	%cl, %eax		C
L(odd):	mov	%eax, %edi		C
	mov	%edx, %ecx		C
	sub	%eax, %ecx		C
	sub	%edx, %eax		C
	jnz	L(top)			C

L(end):	mov	%edx, %eax
	pop	%esi
	pop	%edi
	ret

L(shift_alot):
	shr	$MAXSHIFT, %eax
	mov	%eax, %ecx
	jmp	L(mid)
EPILOGUE()
ASM_END()
