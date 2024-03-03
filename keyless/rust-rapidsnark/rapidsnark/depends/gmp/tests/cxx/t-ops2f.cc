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

#include "t-ops2.h"

void checkf (){
  ASSERT_ALWAYS(sqrt(mpf_class(7))>2.64);
  ASSERT_ALWAYS(sqrt(mpf_class(7))<2.65);
  ASSERT_ALWAYS(sqrt(mpf_class(0))==0);
  // TODO: add some consistency checks, as described in
  // https://gmplib.org/list-archives/gmp-bugs/2013-February/002940.html
  CHECK1(mpf_class,1.9,trunc);
  CHECK1(mpf_class,1.9,floor);
  CHECK1(mpf_class,1.9,ceil);
  CHECK1(mpf_class,4.3,trunc);
  CHECK1(mpf_class,4.3,floor);
  CHECK1(mpf_class,4.3,ceil);
  CHECK1(mpf_class,-7.1,trunc);
  CHECK1(mpf_class,-7.1,floor);
  CHECK1(mpf_class,-7.1,ceil);
  CHECK1(mpf_class,-2.8,trunc);
  CHECK1(mpf_class,-2.8,floor);
  CHECK1(mpf_class,-2.8,ceil);
  CHECK1(mpf_class,-1.5,trunc);
  CHECK1(mpf_class,-1.5,floor);
  CHECK1(mpf_class,-1.5,ceil);
  CHECK1(mpf_class,2.5,trunc);
  CHECK1(mpf_class,2.5,floor);
  CHECK1(mpf_class,2.5,ceil);
  ASSERT_ALWAYS(hypot(mpf_class(-3),mpf_class(4))>4.9);
  ASSERT_ALWAYS(hypot(mpf_class(-3),mpf_class(4))<5.1);
  ASSERT_ALWAYS(hypot(mpf_class(-3),4.)>4.9);
  ASSERT_ALWAYS(hypot(-3.,mpf_class(4))<5.1);
  ASSERT_ALWAYS(hypot(mpf_class(-3),4l)>4.9);
  ASSERT_ALWAYS(hypot(-3l,mpf_class(4))<5.1);
  ASSERT_ALWAYS(hypot(mpf_class(-3),4ul)>4.9);
  ASSERT_ALWAYS(hypot(3ul,mpf_class(4))<5.1);
  CHECK(mpf_class,mpq_class,1.5,2.25,+);
  CHECK(mpf_class,mpq_class,1.5,2.25,-);
  CHECK(mpf_class,mpq_class,1.5,-2.25,*);
  CHECK(mpf_class,mpq_class,1.5,-2,/);
  CHECK_MPQ(mpf_class,-5.5,-2.25,+);
  CHECK_MPQ(mpf_class,-5.5,-2.25,-);
  CHECK_MPQ(mpf_class,-5.5,-2.25,*);
  CHECK_MPQ(mpf_class,-5.25,-0.5,/);
  CHECK_MPQ(mpf_class,5,-2,<);
  CHECK_MPQ(mpf_class,5,-2,>);
  CHECK_MPQ(mpf_class,5,-2,<=);
  CHECK_MPQ(mpf_class,5,-2,>=);
  CHECK_MPQ(mpf_class,5,-2,==);
  CHECK_MPQ(mpf_class,5,-2,!=);
  CHECK_MPQ(mpf_class,0,0,<);
  CHECK_MPQ(mpf_class,0,0,>);
  CHECK_MPQ(mpf_class,0,0,<=);
  CHECK_MPQ(mpf_class,0,0,>=);
  CHECK_MPQ(mpf_class,0,0,==);
  CHECK_MPQ(mpf_class,0,0,!=);
}

int
main (void)
{
  tests_start();

  // Enough precision for 1 + denorm_min
  mpf_set_default_prec(DBL_MANT_DIG-DBL_MIN_EXP+42);
  checkf();

  tests_end();
  return 0;
}
