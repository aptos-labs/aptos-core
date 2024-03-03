dnl  AMD64 logops.

dnl  Copyright 2004-2017 Free Software Foundation, Inc.

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


C		c/l	c/l	c/l	good
C	       var-1   var-2   var-3  for cpu?
C AMD K8,K9
C AMD K10	 1.52	 1.75	 1.75	 n
C AMD bd1
C AMD bd2
C AMD bd3
C AMD bd4
C AMD bt1	 2.67	~2.79	~2.79	 =
C AMD bt2	 2.15	 2.65	 2.65	 n
C AMD zen	 1.5	 1.5	 1.5	 =
C Intel P4
C Intel PNR	 2.0	 2.0	 2.0	 =
C Intel NHM	 2.0	 2.0	 2.0	 =
C Intel SBR	 1.5	 1.5	 1.5	 y
C Intel IBR	 1.47	 1.48	 1.48	 y
C Intel HWL	 1.11	 1.35	 1.35	 y
C Intel BWL	 1.09	 1.30	 1.30	 y
C Intel SKL	 1.21	 1.27	 1.27	 y
C Intel atom	 3.31	 3.57	 3.57	 y
C Intel SLM	 3.0	 3.0	 3.0	 =
C VIA nano

ifdef(`OPERATION_and_n',`
  define(`func',`mpn_and_n')
  define(`VARIANT_1')
  define(`LOGOP',`and')')
ifdef(`OPERATION_andn_n',`
  define(`func',`mpn_andn_n')
  define(`VARIANT_2')
  define(`LOGOP',`and')')
ifdef(`OPERATION_nand_n',`
  define(`func',`mpn_nand_n')
  define(`VARIANT_3')
  define(`LOGOP',`and')')
ifdef(`OPERATION_ior_n',`
  define(`func',`mpn_ior_n')
  define(`VARIANT_1')
  define(`LOGOP',`or')')
ifdef(`OPERATION_iorn_n',`
  define(`func',`mpn_iorn_n')
  define(`VARIANT_2')
  define(`LOGOP',`or')')
ifdef(`OPERATION_nior_n',`
  define(`func',`mpn_nior_n')
  define(`VARIANT_3')
  define(`LOGOP',`or')')
ifdef(`OPERATION_xor_n',`
  define(`func',`mpn_xor_n')
  define(`VARIANT_1')
  define(`LOGOP',`xor')')
ifdef(`OPERATION_xnor_n',`
  define(`func',`mpn_xnor_n')
  define(`VARIANT_2')
  define(`LOGOP',`xor')')

define(`addptr', `lea	$1($2), $2')

MULFUNC_PROLOGUE(mpn_and_n mpn_andn_n mpn_nand_n mpn_ior_n mpn_iorn_n mpn_nior_n mpn_xor_n mpn_xnor_n)

C INPUT PARAMETERS
define(`rp',`%rdi')
define(`up',`%rsi')
define(`vp',`%rdx')
define(`n',`%rcx')

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()

ifdef(`VARIANT_1',`
	TEXT
	ALIGN(32)
PROLOGUE(func)
	FUNC_ENTRY(4)
	mov	(vp), %r8
	mov	R32(%rcx), R32(%rax)
	and	$3, R32(%rax)
	je	L(b00)
	cmp	$2, R32(%rax)
	jc	L(b01)
	je	L(b10)

L(b11):	LOGOP	(up), %r8
	mov	%r8, (rp)
	inc	n
	addptr(	-8, up)
	addptr(	-8, vp)
	addptr(	-8, rp)
	jmp	L(e11)
L(b10):	add	$2, n
	addptr(	-16, up)
	addptr(	-16, vp)
	addptr(	-16, rp)
	jmp	L(e10)
L(b01):	LOGOP	(up), %r8
	mov	%r8, (rp)
	dec	n
	jz	L(ret)
	addptr(	8, up)
	addptr(	8, vp)
	addptr(	8, rp)

	ALIGN(16)
L(top):	mov	(vp), %r8
L(b00):	mov	8(vp), %r9
	LOGOP	(up), %r8
	LOGOP	8(up), %r9
	mov	%r8, (rp)
	mov	%r9, 8(rp)
L(e11):	mov	16(vp), %r8
L(e10):	mov	24(vp), %r9
	addptr(	32, vp)
	LOGOP	16(up), %r8
	LOGOP	24(up), %r9
	addptr(	32, up)
	mov	%r8, 16(rp)
	mov	%r9, 24(rp)
	addptr(	32, rp)
	sub	$4, n
	jnz	L(top)

L(ret):	FUNC_EXIT()
	ret
EPILOGUE()
')

ifdef(`VARIANT_2',`
	TEXT
	ALIGN(32)
PROLOGUE(func)
	FUNC_ENTRY(4)
	mov	(vp), %r8
	not	%r8
	mov	R32(%rcx), R32(%rax)
	and	$3, R32(%rax)
	je	L(b00)
	cmp	$2, R32(%rax)
	jc	L(b01)
	je	L(b10)

L(b11):	LOGOP	(up), %r8
	mov	%r8, (rp)
	inc	n
	addptr(	-8, up)
	addptr(	-8, vp)
	addptr(	-8, rp)
	jmp	L(e11)
L(b10):	add	$2, n
	addptr(	-16, up)
	addptr(	-16, vp)
	addptr(	-16, rp)
	jmp	L(e10)
L(b01):	LOGOP	(up), %r8
	mov	%r8, (rp)
	dec	n
	jz	L(ret)
	addptr(	8, up)
	addptr(	8, vp)
	addptr(	8, rp)

	ALIGN(16)
L(top):	mov	(vp), %r8
	not	%r8
L(b00):	mov	8(vp), %r9
	not	%r9
	LOGOP	(up), %r8
	LOGOP	8(up), %r9
	mov	%r8, (rp)
	mov	%r9, 8(rp)
L(e11):	mov	16(vp), %r8
	not	%r8
L(e10):	mov	24(vp), %r9
	not	%r9
	addptr(	32, vp)
	LOGOP	16(up), %r8
	LOGOP	24(up), %r9
	addptr(	32, up)
	mov	%r8, 16(rp)
	mov	%r9, 24(rp)
	addptr(	32, rp)
	sub	$4, n
	jnz	L(top)

L(ret):	FUNC_EXIT()
	ret
EPILOGUE()
')

ifdef(`VARIANT_3',`
	TEXT
	ALIGN(32)
PROLOGUE(func)
	FUNC_ENTRY(4)
	mov	(vp), %r8
	mov	R32(%rcx), R32(%rax)
	and	$3, R32(%rax)
	je	L(b00)
	cmp	$2, R32(%rax)
	jc	L(b01)
	je	L(b10)

L(b11):	LOGOP	(up), %r8
	not	%r8
	mov	%r8, (rp)
	inc	n
	addptr(	-8, up)
	addptr(	-8, vp)
	addptr(	-8, rp)
	jmp	L(e11)
L(b10):	add	$2, n
	addptr(	-16, up)
	addptr(	-16, vp)
	addptr(	-16, rp)
	jmp	L(e10)
L(b01):	LOGOP	(up), %r8
	not	%r8
	mov	%r8, (rp)
	dec	n
	jz	L(ret)
	addptr(	8, up)
	addptr(	8, vp)
	addptr(	8, rp)

	ALIGN(16)
L(top):	mov	(vp), %r8
L(b00):	mov	8(vp), %r9
	LOGOP	(up), %r8
	not	%r8
	LOGOP	8(up), %r9
	not	%r9
	mov	%r8, (rp)
	mov	%r9, 8(rp)
L(e11):	mov	16(vp), %r8
L(e10):	mov	24(vp), %r9
	addptr(	32, vp)
	LOGOP	16(up), %r8
	not	%r8
	LOGOP	24(up), %r9
	addptr(	32, up)
	not	%r9
	mov	%r8, 16(rp)
	mov	%r9, 24(rp)
	addptr(	32, rp)
	sub	$4, n
	jnz	L(top)

L(ret):	FUNC_EXIT()
	ret
EPILOGUE()
')
