dnl  AMD64 mpn_hamdist -- hamming distance.

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

C		    cycles/limb
C AMD K8,K9		 n/a
C AMD K10		 3.26
C AMD bd1		 4.2
C AMD bd2		 4.2
C AMD bd3		 ?
C AMD bd4		 ?
C AMD zen		 1.15
C AMD bobcat		 7.29
C AMD jaguar		 2.53
C Intel P4		 n/a
C Intel core2		 n/a
C Intel NHM		 2.03
C Intel SBR		 1.66
C Intel IBR		 1.62
C Intel HWL		 1.50
C Intel BWL		 1.50
C Intel SKL		 1.50
C Intel atom		 n/a
C Intel SLM		 2.55
C VIA nano		 n/a

C TODO
C  * An AVX pshufb based variant should approach 0.5 c/l on Haswell and later
C    Intel hardware.  Perhaps mix such a loop with popcnt instructions.
C  * The random placement of the L0, L1, L2, etc blocks are due to branch
C    shortening.  More work could be done there.
C  * Combine the accumulators rax and rcx into one register to save some
C    bookkeeping and a push/pop pair.  Unfortunately this cause a slight
C    slowdown for at leat NHM and SBR.

define(`up',		`%rdi')
define(`vp',		`%rsi')
define(`n',		`%rdx')

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

define(`sum', `lea	($1,$2), $2')
define(`sum', `add	$1, $2')

ASM_START()
	TEXT
	ALIGN(32)
PROLOGUE(mpn_hamdist)
	FUNC_ENTRY(3)
	push	%rbx
	push	%rbp

	mov	(up), %r10
	xor	(vp), %r10

	mov	R32(n), R32(%r8)
	and	$3, R32(%r8)

	xor	R32(%rcx), R32(%rcx)
	.byte	0xf3,0x49,0x0f,0xb8,0xc2	C popcnt %r10,%rax

	lea	L(tab)(%rip), %r9
ifdef(`PIC',`
	movslq	(%r9,%r8,4), %r8
	add	%r9, %r8
	jmp	*%r8
',`
	jmp	*(%r9,%r8,8)
')

L(3):	mov	8(up), %r10
	mov	16(up), %r11
	xor	8(vp), %r10
	xor	16(vp), %r11
	xor	R32(%rbp), R32(%rbp)
	sub	$4, n
	jle	L(x3)
	mov	24(up), %r8
	mov	32(up), %r9
	add	$24, up
	add	$24, vp
	jmp	L(e3)

L(0):	mov	8(up), %r9
	xor	8(vp), %r9
	mov	16(up), %r10
	mov	24(up), %r11
	xor	R32(%rbx), R32(%rbx)
	xor	16(vp), %r10
	xor	24(vp), %r11
	add	$32, up
	add	$32, vp
	sub	$4, n
	jle	L(x4)

	ALIGN(16)
L(top):
L(e0):	.byte	0xf3,0x49,0x0f,0xb8,0xe9	C popcnt %r9,%rbp
	mov	(up), %r8
	mov	8(up), %r9
	sum(	%rbx, %rax)
L(e3):	.byte	0xf3,0x49,0x0f,0xb8,0xda	C popcnt %r10,%rbx
	xor	(vp), %r8
	xor	8(vp), %r9
	sum(	%rbp, %rcx)
L(e2):	.byte	0xf3,0x49,0x0f,0xb8,0xeb	C popcnt %r11,%rbp
	mov	16(up), %r10
	mov	24(up), %r11
	add	$32, up
	sum(	%rbx, %rax)
L(e1):	.byte	0xf3,0x49,0x0f,0xb8,0xd8	C popcnt %r8,%rbx
	xor	16(vp), %r10
	xor	24(vp), %r11
	add	$32, vp
	sum(	%rbp, %rcx)
	sub	$4, n
	jg	L(top)

L(x4):	.byte	0xf3,0x49,0x0f,0xb8,0xe9	C popcnt %r9,%rbp
	sum(	%rbx, %rax)
L(x3):	.byte	0xf3,0x49,0x0f,0xb8,0xda	C popcnt %r10,%rbx
	sum(	%rbp, %rcx)
	.byte	0xf3,0x49,0x0f,0xb8,0xeb	C popcnt %r11,%rbp
	sum(	%rbx, %rax)
	sum(	%rbp, %rcx)
L(x2):	add	%rcx, %rax
L(x1):	pop	%rbp
	pop	%rbx
	FUNC_EXIT()
	ret

L(2):	mov	8(up), %r11
	xor	8(vp), %r11
	sub	$2, n
	jle	L(n2)
	mov	16(up), %r8
	mov	24(up), %r9
	xor	R32(%rbx), R32(%rbx)
	xor	16(vp), %r8
	xor	24(vp), %r9
	add	$16, up
	add	$16, vp
	jmp	L(e2)
L(n2):	.byte	0xf3,0x49,0x0f,0xb8,0xcb	C popcnt %r11,%rcx
	jmp	L(x2)

L(1):	dec	n
	jle	L(x1)
	mov	8(up), %r8
	mov	16(up), %r9
	xor	8(vp), %r8
	xor	16(vp), %r9
	xor	R32(%rbp), R32(%rbp)
	mov	24(up), %r10
	mov	32(up), %r11
	add	$40, up
	add	$8, vp
	jmp	L(e1)

EPILOGUE()
	JUMPTABSECT
	ALIGN(8)
L(tab):	JMPENT(	L(0), L(tab))
	JMPENT(	L(1), L(tab))
	JMPENT(	L(2), L(tab))
	JMPENT(	L(3), L(tab))
