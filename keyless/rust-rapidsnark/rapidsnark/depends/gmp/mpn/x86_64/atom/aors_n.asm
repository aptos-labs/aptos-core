dnl  X86-64 mpn_add_n, mpn_sub_n, optimised for Intel Atom.

dnl  Copyright 2011, 2017 Free Software Foundation, Inc.

dnl  Contributed to the GNU project by Marco Bodrato.  Ported to 64-bit by
dnl  Torbjörn Granlund.

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

C	    cycles/limb
C AMD K8,K9	 2
C AMD K10	 2
C AMD bull	 2.34\2.63
C AMD pile	 2.27\2.52
C AMD steam
C AMD excavator
C AMD bobcat	 2.79
C AMD jaguar	 2.78
C Intel P4	11
C Intel core2	 7.5
C Intel NHM	 8.5
C Intel SBR	 2.11
C Intel IBR	 2.07
C Intel HWL	 1.75
C Intel BWL	 1.51
C Intel SKL	 1.52
C Intel atom	 3
C Intel SLM	 4
C VIA nano

define(`rp',	`%rdi')	C rcx
define(`up',	`%rsi')	C rdx
define(`vp',	`%rdx')	C r8
define(`n',	`%rcx')	C r9
define(`cy',	`%r8')	C rsp+40    (mpn_add_nc and mpn_sub_nc)

ifdef(`OPERATION_add_n', `
  define(ADCSBB,    adc)
  define(func_n,    mpn_add_n)
  define(func_nc,   mpn_add_nc)')
ifdef(`OPERATION_sub_n', `
  define(ADCSBB,    sbb)
  define(func_n,    mpn_sub_n)
  define(func_nc,   mpn_sub_nc)')

MULFUNC_PROLOGUE(mpn_add_n mpn_add_nc mpn_sub_n mpn_sub_nc)

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(16)
PROLOGUE(func_n)
	FUNC_ENTRY(4)
	xor	cy, cy			C carry

L(com):	shr	n			C n >> 1
	jz	L(1)			C n == 1
	jc	L(1m2)			C n % 2 == 1

L(0m2):	shr	cy
	mov	(up), %r10
	lea	8(up), up
	lea	8(vp), vp
	lea	-8(rp), rp
	jmp	L(mid)

L(1):	shr	cy
	mov	(up), %r9
	jmp	L(end)

L(1m2):	shr	cy
	mov	(up), %r9

	ALIGN(16)
L(top):	ADCSBB	(vp), %r9
	lea	16(up), up
	mov	-8(up), %r10
	lea	16(vp), vp
	mov	%r9, (rp)
L(mid):	ADCSBB	-8(vp), %r10
	lea	16(rp), rp
	dec	n
	mov	(up), %r9
	mov	%r10, -8(rp)
	jnz	L(top)

L(end):	ADCSBB	(vp), %r9
	mov	$0, R32(%rax)
	mov	%r9, (rp)
	adc	R32(%rax), R32(%rax)
	FUNC_EXIT()
	ret
EPILOGUE()

PROLOGUE(func_nc)
	FUNC_ENTRY(4)
IFDOS(`	mov	56(%rsp), cy	')
	jmp	L(com)
EPILOGUE()
ASM_END()
