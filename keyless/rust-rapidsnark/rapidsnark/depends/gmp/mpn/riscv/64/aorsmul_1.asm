dnl  RISC-V/64 mpn_addmul_1 and mpn_submul_1.

dnl  Copyright 2016 Free Software Foundation, Inc.

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

C  INPUT PARAMETERS
define(`rp',	`a0')
define(`up',	`a1')
define(`n',	`a2')
define(`v0',	`a3')

ifdef(`OPERATION_addmul_1',`
    define(`ADDSUB',	`add')
    define(`CMPCY',	`sltu	$1, $2, $3')
    define(`func',	`mpn_addmul_1')
')
ifdef(`OPERATION_submul_1',`
    define(`ADDSUB',	`sub')
    define(`CMPCY',	`sltu	$1, $3, $2')
    define(`func',	`mpn_submul_1')
')

MULFUNC_PROLOGUE(mpn_addmul_1 mpn_submul_1)

ASM_START()
PROLOGUE(func)
	li	a6, 0

L(top):	ld	a7, 0(up)
	addi	up, up, 8	C bookkeeping
	ld	a4, 0(rp)
	addi	rp, rp, 8	C bookkeeping
	mul	a5, a7, v0
	addi	n, n, -1	C bookkeeping
	mulhu	a7, a7, v0
	ADDSUB	a5, a4, a5
	ADDSUB	a6, a5, a6	C cycle 0, 3, ...
	CMPCY(	a4, a5, a4)
	add	a4, a4, a7
	CMPCY(	a5, a6, a5)	C cycle 1, 4, ...
	sd	a6, -8(rp)
	add	a6, a4, a5	C cycle 2, 5, ...
	bne	n, x0, L(top)	C bookkeeping

L(end):	mv	a0, a6
	ret
EPILOGUE()
ASM_END()
