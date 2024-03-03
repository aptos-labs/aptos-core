/* Test mp*_class operators and functions.

Copyright 2011, 2012, 2018 Free Software Foundation, Inc.

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

template<class T>
void checkqf (){
  CHECK_ALL(T,5.,0,+);
  CHECK_ALL(T,5.,0,-);
  CHECK_ALL(T,5.,2,+); CHECK_MPZ(T,5.,2,+);
  CHECK_ALL(T,5.,2,-); CHECK_MPZ(T,5.,2,-);
  CHECK_ALL(T,5.,2,*); CHECK_MPZ(T,5.,2,*);
  CHECK_ALL(T,5.,2,/); CHECK_MPZ(T,5.,2,/);
  CHECK_ALL(T,0.,2,/);
  CHECK_ALL_SIGNS(T,11.,3,+);
  CHECK_ALL_SIGNS(T,11.,3,-);
  CHECK_ALL_SIGNS(T,13.,1,+);
  CHECK_ALL_SIGNS(T,13.,1,-);
  CHECK_ALL_SIGNS(T,11.,3,*);
  CHECK_ALL_SIGNS(T,11.,4,/);
  CHECK_SI(T,LONG_MIN,1,*);
  CHECK_SI(T,0,3,*);
  CHECK_ALL_COMPARISONS(T,5.,2);
  CHECK_ALL_SIGNS_COMPARISONS(T,11.,3);
  CHECK_MPZ(T,5,-2,<);
  CHECK_MPZ(T,5,-2,>);
  CHECK_MPZ(T,5,-2,<=);
  CHECK_MPZ(T,5,-2,>=);
  CHECK_MPZ(T,5,-2,==);
  CHECK_MPZ(T,5,-2,!=);
  CHECK_MPZ(T,0,0,<);
  CHECK_MPZ(T,0,0,>);
  CHECK_MPZ(T,0,0,<=);
  CHECK_MPZ(T,0,0,>=);
  CHECK_MPZ(T,0,0,==);
  CHECK_MPZ(T,0,0,!=);
  ASSERT_ALWAYS(T(6)<<2==6.*4);
  ASSERT_ALWAYS(T(6)>>2==6./4);
  ASSERT_ALWAYS(T(-13)<<2==-13.*4);
  ASSERT_ALWAYS(T(-13)>>2==-13./4);
  ASSERT_ALWAYS(++T(7)==8);
  ASSERT_ALWAYS(++T(-8)==-7);
  ASSERT_ALWAYS(--T(8)==7);
  ASSERT_ALWAYS(--T(-7)==-8);
  ASSERT_ALWAYS(+T(7)==7);
  ASSERT_ALWAYS(+T(-8)==-8);
  ASSERT_ALWAYS(-T(7)==-7);
  ASSERT_ALWAYS(-T(-8)==8);
  ASSERT_ALWAYS(abs(T(7))==7);
  ASSERT_ALWAYS(abs(T(-8))==8);
  ASSERT_ALWAYS(sgn(T(0))==0);
  ASSERT_ALWAYS(sgn(T(9))==1);
  ASSERT_ALWAYS(sgn(T(-17))==-1);
  ASSERT_ALWAYS(T(1)+DBL_MAX>2);
  ASSERT_ALWAYS(T(1)+DBL_MIN>1);
  ASSERT_ALWAYS(T(1)+DBL_MIN<1.001);
  ASSERT_ALWAYS(T(1)+std::numeric_limits<double>::denorm_min()>1);
  ASSERT_ALWAYS(T(1)+std::numeric_limits<double>::denorm_min()<1.001);
}

int
main (void)
{
  tests_start();

  // Enough precision for 1 + denorm_min
  mpf_set_default_prec(DBL_MANT_DIG-DBL_MIN_EXP+42);
  checkqf<mpq_class>();
  checkqf<mpf_class>();

  tests_end();
  return 0;
}
