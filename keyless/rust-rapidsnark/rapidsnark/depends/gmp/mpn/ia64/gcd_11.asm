dnl  Itanium-2 mpn_gcd_11

dnl  Copyright 2002-2005, 2012, 2013, 2015, 2019 Free Software Foundation, Inc.

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


C           cycles/bitpair (1x1 gcd)
C Itanium:       ?
C Itanium 2:     4.5


ASM_START()

C ctz_table[n] is the number of trailing zeros on n, or MAXSHIFT if n==0.

deflit(MAXSHIFT, 7)
deflit(MASK, eval((m4_lshift(1,MAXSHIFT))-1))

	.rodata
	ALIGN(m4_lshift(1,MAXSHIFT))	C align table to allow using dep
ctz_table:
	data1	MAXSHIFT
forloop(i,1,MASK,
`	data1	m4_count_trailing_zeros(i)-1
')

define(`x0', r32)
define(`y0', r33)

PROLOGUE(mpn_gcd_11)
	.prologue
	.body
		addl	r22 = @ltoff(ctz_table), r1
	;;
		ld8	r22 = [r22]
		br	L(ent)
	;;

	ALIGN(32)
L(top):
	.pred.rel "mutex", p6,p7
 {.mmi;	(p7)	mov	y0 = x0
	(p6)	sub	x0 = x0, y0
		dep	r21 = r19, r22, 0, MAXSHIFT	C concat(table,lowbits)
}{.mmi;		and	r20 = MASK, r19
	(p7)	mov	x0 = r19
		and	r23 = 6, r19
	;;
}{.mmi;		cmp.eq	p6,p0 = 4, r23
		cmp.eq	p7,p0 = 0, r23
		shr.u	x0 = x0, 1		C shift-by-1, always OK
}{.mmb;		ld1	r16 = [r21]
		cmp.eq	p10,p0 = 0, r20
	(p10)	br.spnt.few.clr	 L(count_better)
	;;
}
L(bck):
	.pred.rel "mutex", p6,p7
 {.mii;		nop	0
	(p6)	shr.u	x0 = x0, 1		C u was ...100 before shift-by-1 above
	(p7)	shr.u	x0 = x0, r16		C u was ...000 before shift-by-1 above
	;;
}
L(ent):
 {.mmi;		sub	r19 = y0, x0
		cmp.gtu	p6,p7 = x0, y0
		cmp.ne	p8,p0 = x0, y0
}{.mmb;		nop	0
		nop	0
	(p8)	br.sptk.few.clr L(top)
}

L(end):		mov	r8 = y0
		br.ret.sptk.many b0

L(count_better):
		add	r20 = -1, x0
	;;
		andcm	r23 = r20, x0
	;;
		popcnt	r16 = r23
		br	L(bck)
EPILOGUE()
