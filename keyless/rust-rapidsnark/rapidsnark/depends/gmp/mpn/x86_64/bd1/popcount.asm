dnl  AMD64 SSSE3/XOP mpn_popcount -- population count.

dnl  Copyright 2010-2017 Free Software Foundation, Inc.

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

C		    cycles/limb	  good for cpu?
C AMD K8,K9		n/a
C AMD K10		n/a
C AMD bd1		 1.27		y
C AMD bd2		 1.24		y
C AMD bd3		 ?
C AMD bd4		 1.22
C AMD zen		n/a
C AMD bobcat		n/a
C AMD jaguar		n/a
C Intel P4		n/a
C Intel CNR		n/a
C Intel PNR		n/a
C Intel NHM		n/a
C Intel SBR		n/a
C Intel IBR		n/a
C Intel HWL		n/a
C Intel BWL		n/a
C Intel SKL		n/a
C Intel atom		n/a
C Intel SLM		n/a
C VIA nano		n/a

C TODO
C  * We need to use .byte for vpshlb, vpperm, vphaddubq, and all popcnt if we
C    intend to support old systems.

C We use vpshlb and vpperm below, which are XOP extensions to AVX.  Some
C systems, e.g., NetBSD, set OSXSAVE but nevertheless trigger SIGILL for AVX.
C We fall back to the core2 code.
ifdef(`GMP_AVX_NOT_REALLY_AVAILABLE',`
MULFUNC_PROLOGUE(mpn_popcount)
include_mpn(`x86_64/core2/popcount.asm')
',`

define(`up',		`%rdi')
define(`n',		`%rsi')

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(32)
PROLOGUE(mpn_popcount)
	FUNC_ENTRY(3)
	lea	L(cnsts)(%rip), %r9

ifdef(`PIC', `define(`OFF1',32) define(`OFF2',48) define(`OFF3',64)',
	     `define(`OFF1',64) define(`OFF2',80) define(`OFF3',96)')
	movdqa	OFF1`'(%r9), %xmm7	C nibble counts table
	movdqa	OFF2`'(%r9), %xmm6	C splat shift counts
	movdqa	OFF3`'(%r9), %xmm9	C masks
	pxor	%xmm4, %xmm4
	pxor	%xmm5, %xmm5		C 0-reg
	pxor	%xmm8, %xmm8		C grand total count

	xor	R32(%rdx), R32(%rdx)

	mov	R32(n), R32(%rax)
	and	$7, R32(%rax)
ifdef(`PIC',`
	movslq	(%r9,%rax,4), %rax
	add	%r9, %rax
	jmp	*%rax
',`
	jmp	*(%r9,%rax,8)
')

L(1):	.byte	0xf3,0x48,0x0f,0xb8,0x17	C popcnt (up),%rdx
	add	$8, up
	dec	n
	jnz	L(top)
	mov	%rdx, %rax
	FUNC_EXIT()
	ret

L(2):	add	$-48, up
	jmp	L(e2)

L(3):	.byte	0xf3,0x48,0x0f,0xb8,0x17	C popcnt (up), %rdx
	add	$-40, up
	jmp	L(e2)

L(4):	add	$-32, up
	jmp	L(e4)

L(5):	.byte	0xf3,0x48,0x0f,0xb8,0x17	C popcnt (up), %rdx
	add	$-24, up
	jmp	L(e4)

L(6):	add	$-16, up
	jmp	L(e6)

L(7):	.byte	0xf3,0x48,0x0f,0xb8,0x17	C popcnt (up), %rdx
	add	$-8, up
	jmp	L(e6)

	ALIGN(32)
L(top):	lddqu	(up), %xmm0
	.byte	0x8f,0xe9,0x48,0x94,0xc8	C vpshlb %xmm6, %xmm0, %xmm1
	pand	%xmm9, %xmm0
	pand	%xmm9, %xmm1
	.byte	0x8f,0xe8,0x40,0xa3,0xd7,0x00	C vpperm %xmm0,%xmm7,%xmm7,%xmm2
	.byte	0x8f,0xe8,0x40,0xa3,0xdf,0x10	C vpperm %xmm1, %xmm7, %xmm7, %xmm3
	paddb	%xmm2, %xmm3
	paddb	%xmm3, %xmm4
L(e6):	lddqu	16(up), %xmm0
	.byte	0x8f,0xe9,0x48,0x94,0xc8	C vpshlb %xmm6, %xmm0, %xmm1
	pand	%xmm9, %xmm0
	pand	%xmm9, %xmm1
	.byte	0x8f,0xe8,0x40,0xa3,0xd7,0x00	C vpperm %xmm0,%xmm7,%xmm7,%xmm2
	.byte	0x8f,0xe8,0x40,0xa3,0xdf,0x10	C vpperm %xmm1,%xmm7,%xmm7,%xmm3
	paddb	%xmm2, %xmm3
	paddb	%xmm3, %xmm4
L(e4):	lddqu	32(up), %xmm0
	.byte	0x8f,0xe9,0x48,0x94,0xc8	C vpshlb %xmm6, %xmm0, %xmm1
	pand	%xmm9, %xmm0
	pand	%xmm9, %xmm1
	.byte	0x8f,0xe8,0x40,0xa3,0xd7,0x00	C vpperm %xmm0, %xmm7, %xmm7, %xmm2
	.byte	0x8f,0xe9,0x78,0xd3,0xec	C vphaddubq %xmm4, %xmm5
	.byte	0x8f,0xe8,0x40,0xa3,0xe7,0x10	C vpperm %xmm1,%xmm7,%xmm7,%xmm4
	paddb	%xmm2, %xmm4
L(e2):	popcnt	48(up), %r8
	popcnt	56(up), %r9
	add	$64, up
	paddq	%xmm5, %xmm8			C sum to 2 x 64-bit counts
	add	%r8, %rdx
	add	%r9, %rdx
	sub	$8, n
	jg	L(top)

	.byte	0x8f,0xe9,0x78,0xd3,0xec	C vphaddubq %xmm4, %xmm5
	paddq	%xmm5, %xmm8
	pshufd	$14, %xmm8, %xmm0
	paddq	%xmm8, %xmm0
	movq	%xmm0, %rax
	add	%rdx, %rax
	FUNC_EXIT()
	ret
EPILOGUE()
DEF_OBJECT(L(cnsts),16,`JUMPTABSECT')
	JMPENT(	L(top), L(cnsts))
	JMPENT(	L(1), L(cnsts))
	JMPENT(	L(2), L(cnsts))
	JMPENT(	L(3), L(cnsts))
	JMPENT(	L(4), L(cnsts))
	JMPENT(	L(5), L(cnsts))
	JMPENT(	L(6), L(cnsts))
	JMPENT(	L(7), L(cnsts))
	.byte	0x00,0x01,0x01,0x02,0x01,0x02,0x02,0x03
	.byte	0x01,0x02,0x02,0x03,0x02,0x03,0x03,0x04
	.byte	-4,-4,-4,-4,-4,-4,-4,-4
	.byte	-4,-4,-4,-4,-4,-4,-4,-4
	.byte	0x0f,0x0f,0x0f,0x0f,0x0f,0x0f,0x0f,0x0f
	.byte	0x0f,0x0f,0x0f,0x0f,0x0f,0x0f,0x0f,0x0f
END_OBJECT(L(cnsts))
')
