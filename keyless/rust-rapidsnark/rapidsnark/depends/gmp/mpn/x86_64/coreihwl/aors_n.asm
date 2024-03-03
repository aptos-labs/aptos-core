dnl  AMD64 mpn_add_n, mpn_sub_n

dnl  Copyright 2017 Free Software Foundation, Inc.

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

C	     cycles/limb
C AMD K8,K9
C AMD K10
C AMD bd1	 1.5  with fluctuations
C AMD bd2	 1.5  with fluctuations
C AMD bd3
C AMD bd4	 1.6
C AMD zen
C AMD bt1
C AMD bt2
C Intel P4
C Intel PNR
C Intel NHM
C Intel SBR
C Intel IBR
C Intel HWL	 1.21
C Intel BWL	 1.04
C Intel SKL
C Intel atom
C Intel SLM
C VIA nano

C The loop of this code is the result of running a code generation and
C optimization tool suite written by David Harvey and Torbjorn Granlund.

C INPUT PARAMETERS
define(`rp',	`%rdi')	C rcx
define(`up',	`%rsi')	C rdx
define(`vp',	`%rdx')	C r8
define(`n',	`%rcx')	C r9
define(`cy',	`%r8')	C rsp+40    (mpn_add_nc and mpn_sub_nc)

ifdef(`OPERATION_add_n', `
	define(ADCSBB,	      adc)
	define(func,	      mpn_add_n)
	define(func_nc,	      mpn_add_nc)')
ifdef(`OPERATION_sub_n', `
	define(ADCSBB,	      sbb)
	define(func,	      mpn_sub_n)
	define(func_nc,	      mpn_sub_nc)')

MULFUNC_PROLOGUE(mpn_add_n mpn_add_nc mpn_sub_n mpn_sub_nc)

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(16)
PROLOGUE(func_nc)
	FUNC_ENTRY(4)
IFDOS(`	mov	56(%rsp), %r8	')

	mov	R32(n), R32(%rax)
	shr	$3, n
	and	$7, R32(%rax)

	lea	L(tab)(%rip), %r9
	neg	%r8			C set carry
ifdef(`PIC',`
	movslq	(%r9,%rax,4), %rax
	lea	(%r9,%rax), %rax	C lea not add to preserve carry
	jmp	*%rax
',`
	jmp	*(%r9,%rax,8)
')
EPILOGUE()

	ALIGN(16)
PROLOGUE(func)
	FUNC_ENTRY(4)

	mov	R32(n), R32(%rax)
	shr	$3, n
	and	$7, R32(%rax)		C clear cy as side-effect

	lea	L(tab)(%rip), %r9
ifdef(`PIC',`
	movslq	(%r9,%rax,4), %rax
	lea	(%r9,%rax), %rax	C lea not add to preserve carry
	jmp	*%rax
',`
	jmp	*(%r9,%rax,8)
')

L(0):	mov	(up), %r8
	mov	8(up), %r9
	ADCSBB	(vp), %r8
	jmp	L(e0)

L(4):	mov	(up), %r8
	mov	8(up), %r9
	ADCSBB	(vp), %r8
	lea	-32(up), up
	lea	-32(vp), vp
	lea	-32(rp), rp
	inc	n
	jmp	L(e4)

L(5):	mov	(up), %r11
	mov	8(up), %r8
	mov	16(up), %r9
	ADCSBB	(vp), %r11
	lea	-24(up), up
	lea	-24(vp), vp
	lea	-24(rp), rp
	inc	n
	jmp	L(e5)

L(6):	mov	(up), %r10
	ADCSBB	(vp), %r10
	mov	8(up), %r11
	lea	-16(up), up
	lea	-16(vp), vp
	lea	-16(rp), rp
	inc	n
	jmp	L(e6)

L(7):	mov	(up), %r9
	mov	8(up), %r10
	ADCSBB	(vp), %r9
	ADCSBB	8(vp), %r10
	lea	-8(up), up
	lea	-8(vp), vp
	lea	-8(rp), rp
	inc	n
	jmp	L(e7)

	ALIGN(16)
L(top):
L(e3):	mov	%r9, 40(rp)
L(e2):	mov	%r10, 48(rp)
L(e1):	mov	(up), %r8
	mov	8(up), %r9
	ADCSBB	(vp), %r8
	mov	%r11, 56(rp)
	lea	64(rp), rp
L(e0):	mov	16(up), %r10
	ADCSBB	8(vp), %r9
	ADCSBB	16(vp), %r10
	mov	%r8, (rp)
L(e7):	mov	24(up), %r11
	mov	%r9, 8(rp)
L(e6):	mov	32(up), %r8
	mov	40(up), %r9
	ADCSBB	24(vp), %r11
	mov	%r10, 16(rp)
L(e5):	ADCSBB	32(vp), %r8
	mov	%r11, 24(rp)
L(e4):	mov	48(up), %r10
	mov	56(up), %r11
	mov	%r8, 32(rp)
	lea	64(up), up
	ADCSBB	40(vp), %r9
	ADCSBB	48(vp), %r10
	ADCSBB	56(vp), %r11
	lea	64(vp), vp
	dec	n
	jnz	L(top)

L(end):	mov	%r9, 40(rp)
	mov	%r10, 48(rp)
	mov	%r11, 56(rp)
	mov	R32(n), R32(%rax)
	adc	R32(n), R32(%rax)
	FUNC_EXIT()
	ret

	ALIGN(16)
L(3):	mov	(up), %r9
	mov	8(up), %r10
	mov	16(up), %r11
	ADCSBB	(vp), %r9
	ADCSBB	8(vp), %r10
	ADCSBB	16(vp), %r11
	jrcxz	L(x3)
	lea	24(up), up
	lea	24(vp), vp
	lea	-40(rp), rp
	jmp	L(e3)
L(x3):	mov	%r9, (rp)
	mov	%r10, 8(rp)
	mov	%r11, 16(rp)
	mov	R32(n), R32(%rax)
	adc	R32(n), R32(%rax)
	FUNC_EXIT()
	ret

	ALIGN(16)
L(1):	mov	(up), %r11
	ADCSBB	(vp), %r11
	jrcxz	L(x1)
	lea	8(up), up
	lea	8(vp), vp
	lea	-56(rp), rp
	jmp	L(e1)
L(x1):	mov	%r11, (rp)
	mov	R32(n), R32(%rax)
	adc	R32(n), R32(%rax)
	FUNC_EXIT()
	ret

	ALIGN(16)
L(2):	mov	(up), %r10
	mov	8(up), %r11
	ADCSBB	(vp), %r10
	ADCSBB	8(vp), %r11
	jrcxz	L(x2)
	lea	16(up), up
	lea	16(vp), vp
	lea	-48(rp), rp
	jmp	L(e2)
L(x2):	mov	%r10, (rp)
	mov	%r11, 8(rp)
	mov	R32(n), R32(%rax)
	adc	R32(n), R32(%rax)
	FUNC_EXIT()
	ret
EPILOGUE()
	JUMPTABSECT
	ALIGN(8)
L(tab):	JMPENT(	L(0), L(tab))
	JMPENT(	L(1), L(tab))
	JMPENT(	L(2), L(tab))
	JMPENT(	L(3), L(tab))
	JMPENT(	L(4), L(tab))
	JMPENT(	L(5), L(tab))
	JMPENT(	L(6), L(tab))
	JMPENT(	L(7), L(tab))
