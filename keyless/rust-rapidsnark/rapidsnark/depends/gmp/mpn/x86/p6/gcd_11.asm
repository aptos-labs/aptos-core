dnl  x86 mpn_gcd_11 optimised for processors with fast BSF.

dnl  Based on the K7 gcd_1.asm, by Kevin Ryde.  Rehacked by Torbjorn Granlund.

dnl  Copyright 2000-2002, 2005, 2009, 2011, 2012, 2015 Free Software
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
C AMD K7	 7.80
C AMD K8,K9	 7.79
C AMD K10	 4.08
C AMD bd1	 ?
C AMD bobcat	 7.82
C Intel P4-2	14.9
C Intel P4-3/4	14.0
C Intel P6/13	 5.09
C Intel core2	 4.22
C Intel NHM	 5.00
C Intel SBR	 5.00
C Intel atom	17.1
C VIA nano	?
C Numbers measured with: speed -CD -s16-32 -t16 mpn_gcd_1


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
	jmp	L(odd)

	ALIGN(16)		C               K10   BD    C2    NHM   SBR
L(top):	cmovc(	%esi, %eax)	C u = |v - u|   0,3   0,3   0,6   0,5   0,5
	cmovc(	%edi, %edx)	C v = min(u,v)  0,3   0,3   2,8   1,7   1,7
	shr	%cl, %eax	C               1,7   1,6   2,8   2,8   2,8
L(odd):	mov	%edx, %esi	C               1     1     4     3     3
	sub	%eax, %esi	C               2     2     5     4     4
	bsf	%esi, %ecx	C               3     3     6     5     5
	mov	%eax, %edi	C               2     2     3     3     4
	sub	%edx, %eax	C               2     2     4     3     4
	jnz	L(top)		C

L(end):	mov	%edx, %eax
	pop	%esi
	pop	%edi
	ret
EPILOGUE()
