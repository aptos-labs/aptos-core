dnl  SPARC64 mpn_gcd_11.

dnl  Based on the K7 gcd_1.asm, by Kevin Ryde.  Rehacked for SPARC by Torbjörn
dnl  Granlund.

dnl  Copyright 2000-2002, 2005, 2009, 2011-2013 Free Software Foundation, Inc.

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


C		  cycles/bit (approx)
C UltraSPARC 1&2:	 5.1
C UltraSPARC 3:		 5.0
C UltraSPARC T1:	11.4
C UltraSPARC T3:	10
C UltraSPARC T4:	 6
C Numbers measured with: speed -CD -s32-64 -t32 mpn_gcd_1

C ctz_table[n] is the number of trailing zeros on n, or MAXSHIFT if n==0.

deflit(MAXSHIFT, 7)
deflit(MASK, eval((m4_lshift(1,MAXSHIFT))-1))

	RODATA
	TYPE(ctz_table,object)
ctz_table:
	.byte	MAXSHIFT
forloop(i,1,MASK,
`	.byte	m4_count_trailing_zeros(i)
')
	SIZE(ctz_table,.-ctz_table)

define(`u0',    `%o0')
define(`v0',    `%o1')

ASM_START()
	REGISTER(%g2,#scratch)
	REGISTER(%g3,#scratch)
PROLOGUE(mpn_gcd_11)
	LEA64(ctz_table, o5, g4)
	b	L(odd)
	 mov	u0, %o4

	ALIGN(16)
L(top):	movcc	%xcc, %o4, v0		C v = min(u,v)
	movcc	%xcc, %o2, %o0		C u = |v - u]
L(mid):	ldub	[%o5+%g3], %g5		C
	brz,a,pn %g3, L(shift_alot)	C
	 srlx	%o0, MAXSHIFT, %o0
	srlx	%o0, %g5, %o4		C new u, odd
L(odd):	subcc	v0, %o4, %o2		C v - u, set flags for branch and movcc
	sub	%o4, v0, %o0		C u - v
	bnz,pt	%xcc, L(top)		C
	 and	%o2, MASK, %g3		C extract low MAXSHIFT bits from (v-u)

	retl
	 mov	v0, %o0

L(shift_alot):
	b	L(mid)
	 and	%o0, MASK, %g3		C
EPILOGUE()
