/* asl.h -- artificially small limbs support by means of C++ operator
   overloading.

Copyright 2016 Free Software Foundation, Inc.

This file is part of the GNU MP Library.

The GNU MP Library is free software; you can redistribute it and/or modify
it under the terms of either:

  * the GNU Lesser General Public License as published by the Free
    Software Foundation; either version 3 of the License, or (at your
    option) any later version.

or

  * the GNU General Public License as published by the Free Software
    Foundation; either version 2 of the License, or (at your option) any
    later version.

or both in parallel, as here.

The GNU MP Library is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License
for more details.

You should have received copies of the GNU General Public License and the
GNU Lesser General Public License along with the GNU MP Library.  If not,
see https://www.gnu.org/licenses/.  */

#include <iostream>
#include <cstdint>
#include <cstdlib>
// #include <stdexcept>

#ifndef GMP_ASSERT_ALWAYS
#define GMP_ASSERT_ALWAYS(cc) do {if (!(cc)) abort();} while (0)
#endif

// Missing: post++ post-- ++pre --prec bool(limb) !limb

#ifndef GMP_LIMB_BITS
#define GMP_LIMB_BITS 4
#endif

#define GMP_NUMB_MASK (2 * (1ul << (GMP_LIMB_BITS - 1)) - 1)

#define BINOP_MASK(op, type)				\
  mp_limb_t& operator op##=(const type& rhs) {		\
    limbo = (limbo op rhs.limbo) & GMP_NUMB_MASK;	\
    return *this;					\
  }
#define BINOP_NOMASK(op, type)				\
  mp_limb_t& operator op##=(const type& rhs) {		\
    limbo = limbo op rhs.limbo;				\
    return *this;					\
  }

typedef std::conditional<(GMP_NUMB_MASK <= 0xffff), uint16_t, uint32_t >::type type24;
typedef std::conditional<(GMP_NUMB_MASK <= 0xff), uint8_t, type24>::type mtype;

class mp_limb_t {
public:
  mp_limb_t() {}	// put random garbage in limbo?
  mp_limb_t(const unsigned int rhs) { limbo = rhs & GMP_NUMB_MASK; }
  // mp_limb_t(const mp_limb_t& rhs) { limbo = rhs.limbo; } // Causes havoc
  BINOP_MASK(+, mp_limb_t)
  BINOP_MASK(-, mp_limb_t)
  BINOP_MASK(*, mp_limb_t)
  BINOP_NOMASK(/, mp_limb_t)
  BINOP_NOMASK(%, mp_limb_t)
  BINOP_NOMASK(&, mp_limb_t)
  BINOP_NOMASK(|, mp_limb_t)
  BINOP_NOMASK(^, mp_limb_t)
  mp_limb_t& operator<<=(const unsigned int rhs) {
    GMP_ASSERT_ALWAYS (rhs < GMP_LIMB_BITS);
    limbo = (limbo << rhs) & GMP_NUMB_MASK;
    return *this;
  }
  mp_limb_t& operator>>=(const unsigned int rhs) {
    GMP_ASSERT_ALWAYS (rhs < GMP_LIMB_BITS);
    limbo = limbo >> rhs;
    return *this;
  }
  mp_limb_t operator-() {
    return static_cast<mp_limb_t>((-limbo) & GMP_NUMB_MASK);
    // mp_limb_t x;  x.limbo = (-limbo) & GMP_NUMB_MASK;  return x;
  }
  mp_limb_t operator~() {
    return static_cast<mp_limb_t>((~limbo) & GMP_NUMB_MASK);
    // mp_limb_t x;  x.limbo = (~limbo) & GMP_NUMB_MASK;  return x;
  }
  operator unsigned int() const { return limbo; }
  operator          int() const { return limbo; }

#define RELOP(op)							\
  inline bool operator op(const mp_limb_t rhs) {			\
    return limbo op rhs.limbo;						\
  }
  RELOP(==)
  RELOP(!=)
  RELOP(<)
  RELOP(>)
  RELOP(<=)
  RELOP(>=)

private:
  mtype limbo;
};

#define BINOP2(op, type)						\
  inline mp_limb_t operator op(mp_limb_t lhs, const type& rhs) {	\
    lhs op##= rhs;							\
    return lhs;								\
  }

BINOP2(+, mp_limb_t)
BINOP2(-, mp_limb_t)
BINOP2(*, mp_limb_t)
BINOP2(/, mp_limb_t)
BINOP2(%, mp_limb_t)
BINOP2(&, mp_limb_t)
BINOP2(|, mp_limb_t)
BINOP2(^, mp_limb_t)
BINOP2(<<, unsigned int)
BINOP2(>>, unsigned int)
