// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A declaration holding a view of its type argument, at a type holding a mutable
// reference: its fields are read at the argument, so the view holds what is referred to.

pub struct Viewing<'a, T> {
    pub view: &'a T,
}

pub struct Outer<'a, T> {
    pub inner: Viewing<'a, T>,
    pub tag: u8,
}

pub enum Seen<'a, T> {
    Held(&'a T),
    Empty,
}

pub struct Holder<'a> {
    pub r: &'a mut u32,
}

pub fn id<T>(x: T) -> T {
    x
}

pub fn view_inside<T>(x: T) -> T {
    let _viewing = Viewing { view: &x };
    x
}

pub fn view_nested<T>(x: T) -> T {
    {
        let inner = Viewing { view: &x };
        let _outer = Viewing { view: &inner };
    }
    x
}

pub fn view_stored<T>(x: T) -> T {
    let _outer = Outer { inner: Viewing { view: &x }, tag: 1 };
    x
}

pub fn view_enum<T>(x: T, held: bool) -> T {
    let seen = if held { Seen::Held(&x) } else { Seen::Empty };
    match seen {
        Seen::Held(_) => {},
        Seen::Empty => {},
    }
    x
}

pub fn view_passed<T>(x: T) -> T {
    let _viewing = id(Viewing { view: &x });
    x
}

pub fn viewing_reference(v: u32) -> u32 {
    let mut a = v;
    let r = view_inside(&mut a);
    *r = 7;
    a
}

pub fn nested_reference(v: u32) -> u32 {
    let mut a = v;
    let r = view_nested(&mut a);
    *r = 7;
    a
}

pub fn stored_reference(v: u32) -> u32 {
    let mut a = v;
    let r = view_stored(&mut a);
    *r = 7;
    a
}

pub fn enum_reference(v: u32, held: bool) -> u32 {
    let mut a = v;
    let r = view_enum(&mut a, held);
    *r = 7;
    a
}

pub fn passed_reference(v: u32) -> u32 {
    let mut a = v;
    let r = view_passed(&mut a);
    *r = 7;
    a
}

pub fn holder_reference(v: u32) -> u32 {
    let mut a = v;
    let h = view_inside(Holder { r: &mut a });
    *h.r = 7;
    a
}

pub fn viewing_value(v: u32) -> u32 {
    view_stored(view_nested(view_inside(v)))
}

pub fn observed_at_site(v: u32) -> u32 {
    let mut a = v;
    let r = &mut a;
    let seen = {
        let viewing = Viewing { view: &r };
        **viewing.view
    };
    *r = seen;
    a
}

pub fn observed_stored_at_site(v: u32) -> u32 {
    let mut a = v;
    let r = &mut a;
    let seen = {
        let outer = Outer { inner: Viewing { view: &r }, tag: 1 };
        **outer.inner.view
    };
    *r = seen;
    a
}

pub fn observed_enum_at_site(v: u32) -> u32 {
    let mut a = v;
    let r = &mut a;
    let seen = {
        let held = Seen::Held(&r);
        match held {
            Seen::Held(view) => **view,
            Seen::Empty => 0,
        }
    };
    *r = seen;
    a
}

pub fn peek<T>(_x: &T) {}

pub fn take_view<T>(_v: Viewing<'_, T>) {}

pub fn take_seen<T>(_v: Seen<'_, T>) {}

pub fn make_view<T>(x: &T) -> Viewing<'_, T> {
    Viewing { view: x }
}

pub fn view_of_view<T>(x: T) -> T {
    let w = make_view(&x);
    peek(&w);
    take_view(w);
    x
}

pub fn peek_site(v: u32) -> u32 {
    let mut a = v;
    let r = &mut a;
    let w = Viewing { view: &r };
    peek(&w);
    *r = 7;
    a
}

pub fn take_site(v: u32) -> u32 {
    let mut a = v;
    let r = &mut a;
    take_view(Viewing { view: &r });
    *r = 7;
    a
}

pub fn take_seen_site(v: u32) -> u32 {
    let mut a = v;
    let r = &mut a;
    take_seen(Seen::Held(&r));
    *r = 7;
    a
}

pub fn made_site(v: u32) -> u32 {
    let mut a = v;
    let r = &mut a;
    let w = make_view(&r);
    let seen = **w.view;
    *r = seen;
    a
}

pub fn deep_site(v: u32) -> u32 {
    let mut a = v;
    let r = view_of_view(&mut a);
    *r = 7;
    a
}

pub fn body_local_view(v: u32) -> u32 {
    let mut a = v;
    let r = &mut a;
    let seen = {
        let viewing = view_inside(Viewing { view: &r });
        **viewing.view
    };
    *r = seen;
    a
}

pub fn view_through<T>(x: T) -> T {
    view_inside(x)
}

pub fn body_local_through(v: u32) -> u32 {
    let mut a = v;
    let r = &mut a;
    let seen = {
        let viewing = view_through(Viewing { view: &r });
        **viewing.view
    };
    *r = seen;
    a
}
