/* Test mp*_class operators and functions.

Copyright 2011, 2012 Free Software Foundation, Inc.

This file is part of the GNU MP Library test suite.

The GNU MP Library test suite is free software; you can redistribute it
and/or modify it under the terms of the GNU General Public License as
published by the Free Software Foundation; either version 3 of the License,
or (at your option) any later version.

The GNU MP Library test suite is distributed in the hope that it will be
useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General
Public License for more details.

You should have received a copy of the GNU General Public License along with
the GNU MP Library test suite.  If not, see https://www.gnu.org/licenses/.  */

#include "config.h"

#include <math.h>

#include "gmpxx.h"
#include "gmp-impl.h"
#include "tests.h"


#define CHECK1(Type,a,fun) \
  ASSERT_ALWAYS(fun((Type)(a))==fun(a))
#define CHECK(Type1,Type2,a,b,op) \
  ASSERT_ALWAYS(((Type1)(a) op (Type2)(b))==((a) op (b)))
#define CHECK_G(Type,a,b,op) \
  CHECK(Type,Type,a,b,op)
#define CHECK_UI(Type,a,b,op) \
  CHECK(Type,unsigned long,a,b,op); \
  CHECK(unsigned long,Type,a,b,op)
#define CHECK_SI(Type,a,b,op) \
  CHECK(Type,long,a,b,op); \
  CHECK(long,Type,a,b,op)
#define CHECK_D(Type,a,b,op) \
  CHECK(Type,double,a,b,op); \
  CHECK(double,Type,a,b,op)
#define CHECK_MPZ(Type,a,b,op) \
  CHECK(Type,mpz_class,a,b,op); \
  CHECK(mpz_class,Type,a,b,op)
#define CHECK_MPQ(Type,a,b,op) \
  CHECK(Type,mpq_class,a,b,op); \
  CHECK(mpq_class,Type,a,b,op)
#define CHECK_ALL_SIGNED(Type,a,b,op) \
  CHECK_G(Type,a,b,op); \
  CHECK_SI(Type,a,b,op); \
  CHECK_D(Type,a,b,op)
#define CHECK_ALL_SIGNS(Type,a,b,op) \
  CHECK_ALL_SIGNED(Type,a,b,op); \
  CHECK_ALL_SIGNED(Type,-(a),b,op); \
  CHECK_ALL_SIGNED(Type,a,-(b),op); \
  CHECK_ALL_SIGNED(Type,-(a),-(b),op)
#define CHECK_ALL(Type,a,b,op) \
  CHECK_ALL_SIGNED(Type,a,b,op); \
  CHECK_UI(Type,a,b,op)
#define CHECK_ALL_SIGNED_COMPARISONS(Type,a,b) \
  CHECK_ALL_SIGNED(Type,a,b,<); \
  CHECK_ALL_SIGNED(Type,a,b,>); \
  CHECK_ALL_SIGNED(Type,a,b,<=); \
  CHECK_ALL_SIGNED(Type,a,b,>=); \
  CHECK_ALL_SIGNED(Type,a,b,==); \
  CHECK_ALL_SIGNED(Type,a,b,!=)
#define CHECK_ALL_SIGNS_COMPARISONS(Type,a,b) \
  CHECK_ALL_SIGNS(Type,a,b,<); \
  CHECK_ALL_SIGNS(Type,a,b,>); \
  CHECK_ALL_SIGNS(Type,a,b,<=); \
  CHECK_ALL_SIGNS(Type,a,b,>=); \
  CHECK_ALL_SIGNS(Type,a,b,==); \
  CHECK_ALL_SIGNS(Type,a,b,!=)
#define CHECK_ALL_COMPARISONS(Type,a,b) \
  CHECK_ALL(Type,a,b,<); \
  CHECK_ALL(Type,a,b,>); \
  CHECK_ALL(Type,a,b,<=); \
  CHECK_ALL(Type,a,b,>=); \
  CHECK_ALL(Type,a,b,==); \
  CHECK_ALL(Type,a,b,!=)
