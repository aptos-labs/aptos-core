dnl  AMD64 mpn_popcount -- population count.

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
C AMD K10		 1.39
C AMD bd1		 4
C AMD bd2		 4
C AMD bd3		 ?
C AMD bd4		 ?
C AMD zen		 0.72
C AMD bobcat		 5.78
C AMD jaguar		 1.27
C Intel P4		 n/a
C Intel core2		 n/a
C Intel NHM		 1.04
C Intel SBR		 1.02
C Intel IBR		 1.0
C Intel HWL		 1.0
C Intel BWL		 1.0
C Intel SKL		 1.0
C Intel atom		 n/a
C Intel SLM		 1.34
C VIA nano		 n/a

C TODO
C  * We could approach 0.5 c/l for AMD Zen with more unrolling.  That would
C    not cause any additional feed-in overhead as we already use a jump table.
C  * An AVX pshufb based variant should approach 0.5 c/l on Haswell and later
C    Intel hardware.  Perhaps mix such a loop with popcnt instructions.
C  * The random placement of the L0, L1, L2, etc blocks are due to branch
C    shortening.

define(`up',		`%rdi')
define(`n',		`%rsi')

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(32)
PROLOGUE(mpn_popcount)
	FUNC_ENTRY(2)

	mov	R32(n), R32(%r8)
	and	$7, R32(%r8)

	.byte	0xf3,0x48,0x0f,0xb8,0x07	C popcnt (up), %rax
	xor	R32(%rcx), R32(%rcx)

	lea	L(tab)(%rip), %r9
ifdef(`PIC',`
	movslq	(%r9,%r8,4), %r8
	add	%r9, %r8
	jmp	*%r8
',`
	jmp	*(%r9,%r8,8)
')

L(3):	.byte	0xf3,0x4c,0x0f,0xb8,0x57,0x08	C popcnt 8(up), %r10
	.byte	0xf3,0x4c,0x0f,0xb8,0x5f,0x10	C popcnt 16(up), %r11
	add	$24, up
	sub	$8, n
	jg	L(e34)
	add	%r10, %rax
	add	%r11, %rax
L(s1):	FUNC_EXIT()
	ret

L(1):	sub	$8, n
	jle	L(s1)
	.byte	0xf3,0x4c,0x0f,0xb8,0x47,0x08	C popcnt 8(up), %r8
	.byte	0xf3,0x4c,0x0f,0xb8,0x4f,0x10	C popcnt 16(up), %r9
	add	$8, up
	jmp	L(e12)

L(7):	.byte	0xf3,0x4c,0x0f,0xb8,0x57,0x08	C popcnt 0x8(%rdi),%r10
	.byte	0xf3,0x4c,0x0f,0xb8,0x5f,0x10	C popcnt 0x10(%rdi),%r11
	add	$-8, up
	jmp	L(e07)

L(0):	.byte	0xf3,0x48,0x0f,0xb8,0x4f,0x08	C popcnt 0x8(%rdi),%rcx
	.byte	0xf3,0x4c,0x0f,0xb8,0x57,0x10	C popcnt 0x10(%rdi),%r10
	.byte	0xf3,0x4c,0x0f,0xb8,0x5f,0x18	C popcnt 0x18(%rdi),%r11
	jmp	L(e07)

L(4):	.byte	0xf3,0x48,0x0f,0xb8,0x4f,0x08	C popcnt 0x8(%rdi),%rcx
	.byte	0xf3,0x4c,0x0f,0xb8,0x57,0x10	C popcnt 0x10(%rdi),%r10
	.byte	0xf3,0x4c,0x0f,0xb8,0x5f,0x18	C popcnt 0x18(%rdi),%r11
	add	$32, up
	sub	$8, n
	jle	L(x4)

	ALIGN(16)
L(top):
L(e34):	.byte	0xf3,0x4c,0x0f,0xb8,0x07	C popcnt (%rdi),%r8
	.byte	0xf3,0x4c,0x0f,0xb8,0x4f,0x08	C popcnt 0x8(%rdi),%r9
	add	%r10, %rcx
	add	%r11, %rax
L(e12):	.byte	0xf3,0x4c,0x0f,0xb8,0x57,0x10	C popcnt 0x10(%rdi),%r10
	.byte	0xf3,0x4c,0x0f,0xb8,0x5f,0x18	C popcnt 0x18(%rdi),%r11
	add	%r8, %rcx
	add	%r9, %rax
L(e07):	.byte	0xf3,0x4c,0x0f,0xb8,0x47,0x20	C popcnt 0x20(%rdi),%r8
	.byte	0xf3,0x4c,0x0f,0xb8,0x4f,0x28	C popcnt 0x28(%rdi),%r9
	add	%r10, %rcx
	add	%r11, %rax
L(e56):	.byte	0xf3,0x4c,0x0f,0xb8,0x57,0x30	C popcnt 0x30(%rdi),%r10
	.byte	0xf3,0x4c,0x0f,0xb8,0x5f,0x38	C popcnt 0x38(%rdi),%r11
	add	$64, up
	add	%r8, %rcx
	add	%r9, %rax
	sub	$8, n
	jg	L(top)

L(x4):	add	%r10, %rcx
	add	%r11, %rax
L(x2):	add	%rcx, %rax

	FUNC_EXIT()
	ret

L(2):	.byte	0xf3,0x48,0x0f,0xb8,0x4f,0x08	C popcnt 0x8(%rdi),%rcx
	sub	$8, n
	jle	L(x2)
	.byte	0xf3,0x4c,0x0f,0xb8,0x47,0x10	C popcnt 0x10(%rdi),%r8
	.byte	0xf3,0x4c,0x0f,0xb8,0x4f,0x18	C popcnt 0x18(%rdi),%r9
	add	$16, up
	jmp	L(e12)

L(5):	.byte	0xf3,0x4c,0x0f,0xb8,0x47,0x08	C popcnt 0x8(%rdi),%r8
	.byte	0xf3,0x4c,0x0f,0xb8,0x4f,0x10	C popcnt 0x10(%rdi),%r9
	add	$-24, up
	jmp	L(e56)

L(6):	.byte	0xf3,0x48,0x0f,0xb8,0x4f,0x08	C popcnt 0x8(%rdi),%rcx
	.byte	0xf3,0x4c,0x0f,0xb8,0x47,0x10	C popcnt 0x10(%rdi),%r8
	.byte	0xf3,0x4c,0x0f,0xb8,0x4f,0x18	C popcnt 0x18(%rdi),%r9
	add	$-16, up
	jmp	L(e56)
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
