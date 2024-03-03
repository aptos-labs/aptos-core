dnl  AMD64 mpn_addmul_1 and mpn_submul_1 for CPUs with mulx.

dnl  Copyright 2012, 2013, 2017 Free Software Foundation, Inc.

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
C AMD K8,K9	 -
C AMD K10	 -
C AMD bd1	 -
C AMD bd2	 -
C AMD bd3	 -
C AMD bd4	 4.3
C AMD zen	 2
C AMD bt1	 -
C AMD bt2	 -
C Intel P4	 -
C Intel PNR	 -
C Intel NHM	 -
C Intel SBR	 -
C Intel IBR	 -
C Intel HWL	 ?
C Intel BWL	 ?
C Intel SKL	 ?
C Intel atom	 -
C Intel SLM	 -
C VIA nano	 -

define(`rp',      `%rdi')   C rcx
define(`up',      `%rsi')   C rdx
define(`n_param', `%rdx')   C r8
define(`v0_param',`%rcx')   C r9

define(`n',       `%rcx')
define(`v0',      `%rdx')

ifdef(`OPERATION_addmul_1',`
      define(`ADDSUB',        `add')
      define(`ADCSBB',        `adc')
      define(`func',  `mpn_addmul_1')
')
ifdef(`OPERATION_submul_1',`
      define(`ADDSUB',        `sub')
      define(`ADCSBB',        `sbb')
      define(`func',  `mpn_submul_1')
')

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

MULFUNC_PROLOGUE(mpn_addmul_1 mpn_submul_1)

ASM_START()
	TEXT
	ALIGN(16)
PROLOGUE(func)
	FUNC_ENTRY(4)
	mov	(up), %r8

	push	%rbx
	push	%r12
	push	%r13

	lea	(up,n_param,8), up
	lea	-32(rp,n_param,8), rp
	mov	R32(n_param), R32(%rax)
	xchg	v0_param, v0		C FIXME: is this insn fast?

	neg	n

	and	$3, R8(%rax)
	jz	L(b0)
	cmp	$2, R8(%rax)
	jz	L(b2)
	jg	L(b3)

L(b1):	mulx(	%r8, %rbx, %rax)
	sub	$-1, n
	jz	L(wd1)
	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (up,n,8), %r9, %r8
	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 8(up,n,8), %r11, %r10
	test	R32(%rax), R32(%rax)		C clear cy
	jmp	L(lo1)

L(b0):	mulx(	%r8, %r9, %r8)
	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 8(up,n,8), %r11, %r10
	.byte	0xc4,0x62,0x93,0xf6,0x64,0xce,0x10	C mulx 16(up,n,8), %r13, %r12
	xor	R32(%rax), R32(%rax)
	jmp	L(lo0)

L(b3):	mulx(	%r8, %r11, %r10)
	.byte	0xc4,0x62,0x93,0xf6,0x64,0xce,0x08	C mulx 8(up,n,8), %r13, %r12
	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xce,0x10	C mulx 16(up,n,8), %rbx, %rax
	add	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	sub	$-3, n
	jz	L(wd3)
	test	R32(%rax), R32(%rax)		C clear cy
	jmp	L(lo3)

L(b2):	mulx(	%r8, %r13, %r12)
	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xce,0x08	C mulx 8(up,n,8), %rbx, %rax
	add	%r12, %rbx
	adc	$0, %rax
	sub	$-2, n
	jz	L(wd2)
	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (up,n,8), %r9, %r8
	test	R32(%rax), R32(%rax)		C clear cy
	jmp	L(lo2)

L(top):	ADDSUB	%r9, (rp,n,8)
L(lo3):	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (up,n,8), %r9, %r8
	ADCSBB	%r11, 8(rp,n,8)
L(lo2):	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 8(up,n,8), %r11, %r10
	ADCSBB	%r13, 16(rp,n,8)
L(lo1):	.byte	0xc4,0x62,0x93,0xf6,0x64,0xce,0x10	C mulx 16(up,n,8), %r13, %r12
	ADCSBB	%rbx, 24(rp,n,8)
	adc	%rax, %r9
L(lo0):	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xce,0x18	C mulx 24(up,n,8), %rbx, %rax
	adc	%r8, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax		C rax = carry limb
	add	$4, n
	js	L(top)

L(end):	ADDSUB	%r9, (rp)
L(wd3):	ADCSBB	%r11, 8(rp)
L(wd2):	ADCSBB	%r13, 16(rp)
L(wd1):	ADCSBB	%rbx, 24(rp)
	adc	n, %rax
	pop	%r13
	pop	%r12
	pop	%rbx
	FUNC_EXIT()
	ret
EPILOGUE()
ASM_END()
