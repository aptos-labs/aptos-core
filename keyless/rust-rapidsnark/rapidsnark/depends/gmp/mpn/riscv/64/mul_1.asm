dnl  RISC-V/64 mpn_mul_1.

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

ASM_START()
PROLOGUE(mpn_mul_1)
	li	a6, 0

L(top):	ld	a7, 0(up)
	addi	up, up, 8	C bookkeeping
	addi	rp, rp, 8	C bookkeeping
	mul	a5, a7, v0
	addi	n, n, -1	C bookkeeping
	mulhu	a7, a7, v0
	add	a6, a5, a6	C cycle 0, 3, ...
	sltu	a5, a6, a5	C cycle 1, 4, ...
	sd	a6, -8(rp)
	add	a6, a7, a5	C cycle 2, 5, ...
	bne	n, x0, L(top)	C bookkeeping

L(end):	mv	a0, a6
	ret
EPILOGUE()
ASM_END()
