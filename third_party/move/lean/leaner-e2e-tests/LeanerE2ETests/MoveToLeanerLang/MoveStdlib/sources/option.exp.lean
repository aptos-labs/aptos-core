-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! This module defines the Option type and its methods to represent and handle an optional value. -/
leaner module 0x1::option where
  use 0x1::std::mem::replace
  use 0x1::std::vector
  use 0x1::std::vector::empty
  use 0x1::std::vector::singleton

  pragma aborts_if_is_strict

  /--
  Abstraction of a value that may or may not be present.
  -/
  enum Option {Element} has Copy, Drop, Store where
    | None
    | Some (e : Element)

  /--
  The `Option` is in an invalid state for the operation attempted.
  The `Option` is `Some` while it should be `None`.
  -/
  const EOPTION_IS_SET : u64 := 262144

  /--
  The `Option` is in an invalid state for the operation attempted.
  The `Option` is `None` while it should be `Some`.
  -/
  const EOPTION_NOT_SET : u64 := 262145

  /--
  Cannot construct an option from a vector with 2 or more elements.
  -/
  const EOPTION_VEC_TOO_LONG : u64 := 262146

  /--
  Return an empty `Option`
  -/
  public fun none {Element}() -> Option<Element> := new Option<Element>::None {}

  spec none where
    pragma opaque
    aborts_if false
    ensures result == spec_none::<Element>()

  spec fun spec_none {Element}() : Option<Element> :=
    new Option<Element>::None {}

  /--
  Return an `Option` containing `e`
  -/
  public fun some {Element}(e : Element) -> Option<Element> :=
    new Option<Element>::Some { e }

  spec some where
    pragma opaque
    aborts_if false
    ensures result == spec_some(e)

  spec fun spec_some {Element}(e : Element) : Option<Element> :=
    new Option<Element>::Some { e }

  public fun from_vec {Element}(
    mut vec : Vector<Element>
  ) -> Option<Element> := do
    assert!(vec.length <= 1, EOPTION_VEC_TOO_LONG)
    return if vec.is_empty() then
      vec.destroy_empty()
      return new Option<Element>::None {}
    else
      let e := vec.pop_back()
      vec.destroy_empty()
      return new Option<Element>::Some { e }

  spec from_vec where
    aborts_if vec.length > 1

  /--
  Return true if `self` does not hold a value
  -/
  public fun is_none {Element}(self : &Option<Element>) -> Bool := self is None

  spec is_none where
    pragma opaque
    aborts_if false
    ensures result == spec_is_none(self)

  spec fun spec_is_none {Element}(self : Option<Element>) : Bool := self is None

  /--
  Return true if `self` holds a value
  -/
  public fun is_some {Element}(self : &Option<Element>) -> Bool := self is Some

  spec is_some where
    pragma opaque
    aborts_if false
    ensures result == spec_is_some(self)

  spec fun spec_is_some {Element}(self : Option<Element>) : Bool := self is Some

  /--
  Return true if the value in `self` is equal to `e_ref`
  Always returns `false` if `self` does not hold a value
  -/
  public fun contains {Element}(
    self : &Option<Element>, e_ref : &Element
  ) -> Bool :=
    if self is None then false else &self.e == e_ref

  spec contains where
    pragma opaque
    aborts_if false
    ensures result == spec_contains(self, e_ref)

  spec fun spec_contains {Element}(
    self : Option<Element>, e : Element
  ) : Bool :=
    self is Some && self.borrow() == e

  /--
  Return an immutable reference to the value inside `self`
  Aborts if `self` does not hold a value
  -/
  public fun borrow {Element}(self : &Option<Element>) -> &Element :=
    if self is None then abort(EOPTION_NOT_SET) else &self.e

  spec borrow where
    pragma opaque
    aborts_if self.is_none() with EOPTION_NOT_SET
    ensures result == spec_borrow(self)

  spec fun spec_borrow {Element}(self : Option<Element>) : Element :=
    if self is Some then self.e else abort()

  /--
  Return a reference to the value inside `self` if it holds one
  Return `default_ref` if `self` does not hold a value
  -/
  public fun borrow_with_default {Element}(
    self : &Option<Element>, default_ref : &Element
  ) -> &Element :=
    if self is None then default_ref else self.e

  spec borrow_with_default where
    pragma opaque
    aborts_if false
    ensures result == (if self.is_some() then self.borrow() else default_ref)

  /--
  Return the value inside `self` if it holds one
  Return `default` if `self` does not hold a value
  -/
  public fun get_with_default {Element has Copy, Drop}(
    self : &Option<Element>, default : Element
  ) -> Element :=
    if self is None then default else self.e

  spec get_with_default where
    pragma opaque
    aborts_if false
    ensures result == (if self.is_some() then self.borrow() else default)

  /--
  Convert the none option `self` to a some option by adding `e`.
  Aborts if `self` already holds a value
  -/
  public fun fill {Element}(
    self : &mut Option<Element>, e : Element
  ) -> Unit := do
    let «old» := replace(self, new Option<Element>::Some { e })
    assert!(«old» is None, EOPTION_IS_SET)

  spec fill where
    pragma opaque
    aborts_if self.is_some() with EOPTION_IS_SET
    ensures self.is_some()
    ensures self.borrow() == e

  /--
  Convert a `some` option to a `none` by removing and returning the value stored inside `self`
  Aborts if `self` does not hold a value
  -/
  public fun extract {Element}(self : &mut Option<Element>) -> Element := do
    let inner := replace(self, new Option<Element>::None {})
    return if inner is Some then inner.e else abort(EOPTION_NOT_SET)

  spec extract where
    pragma opaque
    aborts_if self.is_none() with EOPTION_NOT_SET
    ensures result == old(self).borrow()
    ensures self.is_none()

  /--
  Return a mutable reference to the value inside `self`
  Aborts if `self` does not hold a value
  -/
  public fun borrow_mut {Element}(
    self : &mut Option<Element>
  ) -> &mut Element :=
    if self is None then abort(EOPTION_NOT_SET)
    else
      let e := &mut self.e
      return e

  spec borrow_mut where
    aborts_if self.is_none() with EOPTION_NOT_SET
    ensures result == self.borrow()
    ensures self == old(self)

  /--
  Swap the old value inside `self` with `e` and return the old value
  Aborts if `self` does not hold a value
  -/
  public fun swap {Element}(
    self : &mut Option<Element>, el : Element
  ) -> Element :=
    if self is None then abort(EOPTION_NOT_SET)
    else
      let e := &mut self.e
      return replace(e, el)

  spec swap where
    pragma opaque
    aborts_if self.is_none() with EOPTION_NOT_SET
    ensures result == old(self).borrow()
    ensures self.is_some()
    ensures self.borrow() == el

  /--
  Swap the old value inside `self` with `e` and return the old value;
  or if there is no old value, fill it with `e`.
  Different from swap(), swap_or_fill() allows for `self` not holding a value.
  -/
  public fun swap_or_fill {Element}(
    self : &mut Option<Element>, e : Element
  ) -> Option<Element> := replace(self, new Option<Element>::Some { e })

  spec swap_or_fill where
    pragma opaque
    aborts_if false
    ensures result == old(self)
    ensures self.borrow() == e

  /--
  Destroys `self.` If `self` holds a value, return it. Returns `default` otherwise
  -/
  public fun destroy_with_default {Element has Drop}(
    self : Option<Element>, default : Element
  ) -> Element :=
    if self is None then default else self.e

  spec destroy_with_default where
    pragma opaque
    aborts_if false
    ensures result == (if self.is_some() then self.borrow() else default)

  /--
  Unpack `self` and return its contents
  Aborts if `self` does not hold a value
  -/
  public fun destroy_some {Element}(self : Option<Element>) -> Element :=
    if self is None then abort(EOPTION_NOT_SET) else self.e

  spec destroy_some where
    pragma opaque
    aborts_if self.is_none() with EOPTION_NOT_SET
    ensures result == self.borrow()

  /--
  Unpack `self`
  Aborts if `self` holds a value
  -/
  public fun destroy_none {Element}(self : Option<Element>) -> Unit :=
    assert!(self is None, EOPTION_IS_SET)

  spec destroy_none where
    pragma opaque
    aborts_if self.is_some() with EOPTION_IS_SET

  /--
  Convert `self` into a vector of length 1 if it is `Some`,
  and an empty vector otherwise
  -/
  public fun to_vec {Element}(self : Option<Element>) -> Vector<Element> :=
    if self is None then vector<Element>[] else singleton(self.e)

  spec to_vec where
    pragma opaque
    aborts_if false
    ensures result
        == (if self.is_some() then vector<Element>[self.borrow()]
        else empty::<Element>())

  -- switch documentation context back to module level
