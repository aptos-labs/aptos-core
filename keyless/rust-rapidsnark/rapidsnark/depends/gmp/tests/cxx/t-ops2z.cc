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

void checkz (){
  CHECK_ALL(mpz_class,5,2,+);
  CHECK_ALL(mpz_class,5,2,-);
  CHECK_ALL(mpz_class,5,2,*);
  CHECK_ALL(mpz_class,5,2,/);
  CHECK_ALL(mpz_class,5,2,%);
  CHECK_ALL_COMPARISONS(mpz_class,5,2);
  CHECK_ALL_SIGNS(mpz_class,11,3,+);
  CHECK_ALL_SIGNS(mpz_class,11,3,-);
  CHECK_ALL_SIGNS(mpz_class,11,3,*);
  CHECK_ALL_SIGNS(mpz_class,11,3,/);
  CHECK_ALL_SIGNS(mpz_class,11,3,%);
  CHECK_ALL_SIGNS(mpz_class,17,2,*);
  CHECK_ALL_SIGNS(mpz_class,17,2,/);
  CHECK_ALL_SIGNS(mpz_class,17,2,%);
  CHECK(unsigned long,mpz_class,5,-2,/);
  CHECK(unsigned long,mpz_class,5,-2,%);
  ASSERT_ALWAYS(7ul/mpz_class(1e35)==0);
  ASSERT_ALWAYS(7ul%mpz_class(1e35)==7);
  ASSERT_ALWAYS(7ul/mpz_class(-1e35)==0);
  ASSERT_ALWAYS(7ul%mpz_class(-1e35)==7);
  CHECK_ALL_SIGNS_COMPARISONS(mpz_class,11,3);
  CHECK_ALL(mpz_class,6,3,&);
  CHECK_ALL(mpz_class,6,3,|);
  CHECK_ALL(mpz_class,6,3,^);
  CHECK(mpz_class,unsigned long,6,2,<<);
  CHECK(mpz_class,unsigned long,6,2,>>);
  ASSERT_ALWAYS((mpz_class(-13)<<(unsigned long)2) == (-13)*4);
  CHECK(mpz_class,unsigned long,-13,2,>>);
  ASSERT_ALWAYS(++mpz_class(7)==8);
  ASSERT_ALWAYS(++mpz_class(-8)==-7);
  ASSERT_ALWAYS(--mpz_class(8)==7);
  ASSERT_ALWAYS(--mpz_class(-7)==-8);
  ASSERT_ALWAYS(~mpz_class(7)==-8);
  ASSERT_ALWAYS(~mpz_class(-8)==7);
  ASSERT_ALWAYS(+mpz_class(7)==7);
  ASSERT_ALWAYS(+mpz_class(-8)==-8);
  ASSERT_ALWAYS(-mpz_class(7)==-7);
  ASSERT_ALWAYS(-mpz_class(-8)==8);
  ASSERT_ALWAYS(abs(mpz_class(7))==7);
  ASSERT_ALWAYS(abs(mpz_class(-8))==8);
  ASSERT_ALWAYS(sqrt(mpz_class(7))==2);
  ASSERT_ALWAYS(sqrt(mpz_class(0))==0);
  ASSERT_ALWAYS(sgn(mpz_class(0))==0);
  ASSERT_ALWAYS(sgn(mpz_class(9))==1);
  ASSERT_ALWAYS(sgn(mpz_class(-17))==-1);
  ASSERT_ALWAYS(mpz_class(1)+DBL_MAX>2);
  ASSERT_ALWAYS(mpz_class(1)+DBL_MIN<2);
  ASSERT_ALWAYS(mpz_class(1)+std::numeric_limits<double>::denorm_min()<2);
  ASSERT_ALWAYS(gcd(mpz_class(6),mpz_class(8))==2);
  ASSERT_ALWAYS(gcd(-mpz_class(6),mpz_class(8))==2);
  ASSERT_ALWAYS(gcd(-mpz_class(6),-mpz_class(8))==2);
  ASSERT_ALWAYS(gcd(mpz_class(6),8.f)==2);
  ASSERT_ALWAYS(gcd(-mpz_class(6),static_cast<unsigned char>(8))==2);
  ASSERT_ALWAYS(gcd(static_cast<long>(-6),mpz_class(5)+3)==2);
  ASSERT_ALWAYS(lcm(mpz_class(6),mpz_class(8))==24);
  ASSERT_ALWAYS(lcm(-mpz_class(6),mpz_class(8))==24);
  ASSERT_ALWAYS(lcm(-mpz_class(6),-mpz_class(8))==24);
  ASSERT_ALWAYS(lcm(mpz_class(6),static_cast<short>(8))==24);
  ASSERT_ALWAYS(lcm(-mpz_class(6),static_cast<unsigned char>(8))==24);
  ASSERT_ALWAYS(lcm(-6.,mpz_class(5)+3)==24);
  ASSERT_ALWAYS(factorial(mpz_class(3))==6);
  ASSERT_ALWAYS(factorial(mpz_class(5)-1)==24);
  ASSERT_ALWAYS(mpz_class::factorial(mpz_class(3))==6);
  ASSERT_ALWAYS(mpz_class::factorial(mpz_class(2)*2)==24);
  ASSERT_ALWAYS(mpz_class::factorial(3)==6);
  ASSERT_ALWAYS(mpz_class::factorial(3ul)==6);
  ASSERT_ALWAYS(mpz_class::factorial(3.f)==6);
  mpz_class ret;
  try { ret=factorial(-mpz_class(3)); ASSERT_ALWAYS(0); }
  catch (std::domain_error&) {}
  try { ret=mpz_class::factorial(-2); ASSERT_ALWAYS(0); }
  catch (std::domain_error&) {}
  try { ret=factorial(mpz_class(1)<<300); ASSERT_ALWAYS(0); }
  catch (std::bad_alloc&) {}
  ASSERT_ALWAYS(mpz_class::primorial(mpz_class(3))==6);
  ASSERT_ALWAYS(mpz_class::primorial(mpz_class(2)*2)==6);
  ASSERT_ALWAYS(mpz_class::primorial(3)==6);
  ASSERT_ALWAYS(mpz_class::primorial(3ul)==6);
  ASSERT_ALWAYS(mpz_class::primorial(3.f)==6);
  try { ret=primorial(-mpz_class(3)); ASSERT_ALWAYS(0); }
  catch (std::domain_error&) {}
  try { ret=mpz_class::primorial(-5); ASSERT_ALWAYS(0); }
  catch (std::domain_error&) {}
  try { ret=primorial(mpz_class(1)<<300); ASSERT_ALWAYS(0); }
  catch (std::bad_alloc&) {}
  ASSERT_ALWAYS(mpz_class::fibonacci(mpz_class(6))==8);
  ASSERT_ALWAYS(mpz_class::fibonacci(mpz_class(2)*2)==3);
  ASSERT_ALWAYS(mpz_class::fibonacci(3)==2);
  ASSERT_ALWAYS(mpz_class::fibonacci(3ul)==2);
  ASSERT_ALWAYS(mpz_class::fibonacci(3.f)==2);
  ASSERT_ALWAYS(fibonacci(-mpz_class(6))==-8);
  ASSERT_ALWAYS(mpz_class::fibonacci(-3)==2);
  try { ret=fibonacci(mpz_class(1)<<300); ASSERT_ALWAYS(0); }
  catch (std::bad_alloc&) {}
}

int
main (void)
{
  tests_start();
  checkz();
  tests_end();
  return 0;
}
