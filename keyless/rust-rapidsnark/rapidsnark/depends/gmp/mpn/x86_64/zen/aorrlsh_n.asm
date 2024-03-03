dnl  AMD64 mpn_addlsh_n, mpn_rsblsh_n.

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

C		     cycles/limb
C AMD K8,K9		n/a
C AMD K10		n/a
C AMD bd1		n/a
C AMD bd2		n/a
C AMD bd3		n/a
C AMD bd4		 2.31
C AMD zen		 1.69
C AMD bt1		n/a
C AMD bt2		n/a
C Intel P4		n/a
C Intel PNR		n/a
C Intel NHM		n/a
C Intel SBR		n/a
C Intel IBR		n/a
C Intel HWL		 2.08
C Intel BWL		 1.78
C Intel SKL		 1.78
C Intel atom		n/a
C Intel SLM		n/a
C VIA nano		n/a

C TODO
C  * The loop sustains 4 insns/cycle on zen.
C  * Perhaps avoid using jrcxz by using dec n + jnz.

define(`rp',	`%rdi')
define(`up',	`%rsi')
define(`vp',	`%rdx')
define(`n',	`%rcx')
define(`cnt',	`%r8')

define(`tnc',	`%r9')

ifdef(`OPERATION_addlsh_n',`
  define(ADCSBB,       `adc')
  define(func, mpn_addlsh_n)
')
ifdef(`OPERATION_rsblsh_n',`
  define(ADCSBB,       `sbb')
  define(func, mpn_rsblsh_n)
')

MULFUNC_PROLOGUE(mpn_addlsh_n mpn_rsblsh_n)

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(32)
PROLOGUE(func)
	FUNC_ENTRY(4)
IFDOS(`	mov	56(%rsp), %r8d	')

	mov	(vp), %r10

	mov	R32(n), R32(%rax)
	shr	$3, n
	xor	R32(tnc), R32(tnc)
	sub	cnt, tnc
	and	$7, R32(%rax)

	lea	L(tab)(%rip), %r11
ifdef(`PIC',`
	movslq	(%r11,%rax,4), %rax
	add	%r11, %rax
	jmp	*%rax
',`
	jmp	*(%r11,%rax,8)
')

L(0):	lea	32(up), up
	lea	32(vp), vp
	lea	32(rp), rp
	xor	R32(%r11), R32(%r11)
	jmp	L(e0)

L(7):	mov	%r10, %r11
	lea	24(up), up
	lea	24(vp), vp
	lea	24(rp), rp
	xor	R32(%r10), R32(%r10)
	jmp	L(e7)

L(6):	lea	16(up), up
	lea	16(vp), vp
	lea	16(rp), rp
	xor	R32(%r11), R32(%r11)
	jmp	L(e6)

L(5):	mov	%r10, %r11
	lea	8(up), up
	lea	8(vp), vp
	lea	8(rp), rp
	xor	R32(%r10), R32(%r10)
	jmp	L(e5)

L(end):	ADCSBB	24(up), %rax
	mov	%rax, -40(rp)
	shrx(	tnc, %r11, %rax)
	ADCSBB	n, %rax
	FUNC_EXIT()
	ret

	ALIGN(32)
L(top):	jrcxz	L(end)
	mov	-32(vp), %r10
	ADCSBB	24(up), %rax
	lea	64(up), up
	shrx(	tnc, %r11, %r11)
	mov	%rax, -40(rp)
L(e0):	dec	n
	shlx(	cnt, %r10, %rax)
	lea	(%r11,%rax), %rax
	mov	-24(vp), %r11
	ADCSBB	-32(up), %rax
	shrx(	tnc, %r10, %r10)
	mov	%rax, -32(rp)
L(e7):	shlx(	cnt, %r11, %rax)
	lea	(%r10,%rax), %rax
	mov	-16(vp), %r10
	ADCSBB	-24(up), %rax
	shrx(	tnc, %r11, %r11)
	mov	%rax, -24(rp)
L(e6):	shlx(	cnt, %r10, %rax)
	lea	(%r11,%rax), %rax
	mov	-8(vp), %r11
	ADCSBB	-16(up), %rax
	shrx(	tnc, %r10, %r10)
	mov	%rax, -16(rp)
L(e5):	shlx(	cnt, %r11, %rax)
	lea	(%r10,%rax), %rax
	mov	(vp), %r10
	ADCSBB	-8(up), %rax
	shrx(	tnc, %r11, %r11)
	mov	%rax, -8(rp)
L(e4):	shlx(	cnt, %r10, %rax)
	lea	(%r11,%rax), %rax
	mov	8(vp), %r11
	ADCSBB	(up), %rax
	shrx(	tnc, %r10, %r10)
	mov	%rax, (rp)
L(e3):	shlx(	cnt, %r11, %rax)
	lea	(%r10,%rax), %rax
	mov	16(vp), %r10
	ADCSBB	8(up), %rax
	shrx(	tnc, %r11, %r11)
	mov	%rax, 8(rp)
L(e2):	shlx(	cnt, %r10, %rax)
	lea	(%r11,%rax), %rax
	mov	24(vp), %r11
	ADCSBB	16(up), %rax
	lea	64(vp), vp
	shrx(	tnc, %r10, %r10)
	mov	%rax, 16(rp)
	lea	64(rp), rp
L(e1):	shlx(	cnt, %r11, %rax)
	lea	(%r10,%rax), %rax
	jmp	L(top)

L(4):	xor	R32(%r11), R32(%r11)
	jmp	L(e4)

L(3):	mov	%r10, %r11
	lea	-8(up), up
	lea	-8(vp), vp
	lea	-8(rp), rp
	xor	R32(%r10), R32(%r10)
	jmp	L(e3)

L(2):	lea	-16(up), up
	lea	-16(vp), vp
	lea	-16(rp), rp
	xor	R32(%r11), R32(%r11)
	jmp	L(e2)

L(1):	mov	%r10, %r11
	lea	-24(up), up
	lea	40(vp), vp
	lea	40(rp), rp
	xor	R32(%r10), R32(%r10)
	jmp	L(e1)
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
