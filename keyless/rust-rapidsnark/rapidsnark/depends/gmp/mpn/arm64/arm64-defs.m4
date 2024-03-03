divert(-1)

dnl  m4 macros for ARM64 ELF assembler.

dnl  Copyright 2020 Free Software Foundation, Inc.

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


dnl  Standard commenting is with @, the default m4 # is for constants and we
dnl  don't want to disable macro expansions in or after them.

changecom


dnl  LEA_HI(reg,gmp_symbol), LEA_LO(reg,gmp_symbol)
dnl
dnl  Load the address of gmp_symbol into a register. We split this into two
dnl  parts to allow separation for manual insn scheduling.

ifdef(`PIC',`dnl
define(`LEA_HI', `adrp	$1, :got:$2')dnl
define(`LEA_LO', `ldr	$1, [$1, #:got_lo12:$2]')dnl
',`dnl
define(`LEA_HI', `adrp	$1, $2')dnl
define(`LEA_LO', `add	$1, $1, :lo12:$2')dnl
')dnl

divert`'dnl
