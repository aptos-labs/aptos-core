// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A type argument holding a mutable reference: the callee is read at the substituted
// rows, so a write through what it returns is seen by the lender. Each function has a
// false twin and a vacuous twin, whose clauses are reported here.

pub fn id<T>(x: T) -> T {
    x
}

pub struct Wrapper<T> {
    pub inner: T,
}

pub fn wrap<T>(inner: T) -> Wrapper<T> {
    Wrapper { inner }
}

pub fn unwrap<T>(wrapper: Wrapper<T>) -> T {
    wrapper.inner
}

pub fn some<T>(x: T) -> Option<T> {
    Some(x)
}

pub fn get<T>(x: Option<T>) -> T {
    match x {
        Some(payload) => payload,
        None => panic!(),
    }
}

pub fn outer<T>(wrapper: Wrapper<Wrapper<T>>) -> Wrapper<T> {
    wrapper.inner
}

pub fn twice<T>(x: T) -> T {
    id(id(x))
}

pub fn generic_lender<T>(x: &mut T) -> &mut T {
    id(x)
}

pub fn consume<T>(_x: T) {}

pub struct Viewing<'a, T> {
    pub view: &'a T,
}

pub fn view_inside<T>(x: T) -> T {
    let _viewing = Viewing { view: &x };
    x
}

pub fn observe<T>(x: T) -> T {
    let _view = &x;
    x
}

pub fn opaque_id<T>(x: T) -> T {
    x
}

pub fn recursive_id<T>(n: u32, x: T) -> T {
    if n == 0 {
        x
    } else {
        recursive_id(n - 1, x)
    }
}

pub fn id_write(v: u32) -> u32 {
    let mut a = v;
    let r = id(&mut a);
    *r = 7;
    a
}

pub fn id_write_false(v: u32) -> u32 {
    let mut a = v;
    let r = id(&mut a);
    *r = 7;
    a
}

pub fn id_write_vacuous(v: u32) -> u32 {
    let mut a = v;
    let r = id(&mut a);
    *r = 7;
    a
}

pub fn id_lender(a: &mut u32) {
    let r = id(&mut *a);
    *r = 7;
}

pub fn id_lender_false(a: &mut u32) {
    let r = id(&mut *a);
    *r = 7;
}

pub fn id_at_value(v: u32) -> u32 {
    id(v)
}

pub fn wrapper_field(v: u32) -> u32 {
    let mut a = v;
    let w = wrap(&mut a);
    *w.inner = 7;
    a
}

pub fn wrapper_field_false(v: u32) -> u32 {
    let mut a = v;
    let w = wrap(&mut a);
    *w.inner = 7;
    a
}

pub fn wrapper_field_vacuous(v: u32) -> u32 {
    let mut a = v;
    let w = wrap(&mut a);
    *w.inner = 7;
    a
}

pub fn wrapper_through(v: u32) -> u32 {
    let mut a = v;
    let r = unwrap(wrap(&mut a));
    *r = 7;
    a
}

pub fn wrapper_through_false(v: u32) -> u32 {
    let mut a = v;
    let r = unwrap(wrap(&mut a));
    *r = 7;
    a
}

pub fn wrapper_through_vacuous(v: u32) -> u32 {
    let mut a = v;
    let r = unwrap(wrap(&mut a));
    *r = 7;
    a
}

pub fn option_write(v: u32) -> u32 {
    let mut a = v;
    match some(&mut a) {
        Some(r) => *r = 7,
        None => {},
    }
    a
}

pub fn option_write_false(v: u32) -> u32 {
    let mut a = v;
    match some(&mut a) {
        Some(r) => *r = 7,
        None => {},
    }
    a
}

pub fn option_write_vacuous(v: u32) -> u32 {
    let mut a = v;
    match some(&mut a) {
        Some(r) => *r = 7,
        None => {},
    }
    a
}

pub fn get_write(v: u32, w: u32) -> u32 {
    let mut a = v;
    let mut b = w;
    let r = get(Some(&mut a));
    *r = 7;
    a + b
}

pub fn get_write_false(v: u32, w: u32) -> u32 {
    let mut a = v;
    let mut b = w;
    let r = get(Some(&mut a));
    *r = 7;
    a + b
}

pub fn get_write_vacuous(v: u32, w: u32) -> u32 {
    let mut a = v;
    let mut b = w;
    let r = get(Some(&mut a));
    *r = 7;
    a + b
}

pub fn twice_write(v: u32) -> u32 {
    let mut a = v;
    let r = twice(&mut a);
    *r = 7;
    a
}

pub fn twice_write_false(v: u32) -> u32 {
    let mut a = v;
    let r = twice(&mut a);
    *r = 7;
    a
}

pub fn twice_write_vacuous(v: u32) -> u32 {
    let mut a = v;
    let r = twice(&mut a);
    *r = 7;
    a
}

pub fn generic_at_reference(v: u32) -> u32 {
    let mut a = v;
    let r = generic_lender(&mut a);
    *r = 7;
    a
}

pub fn generic_at_reference_false(v: u32) -> u32 {
    let mut a = v;
    let r = generic_lender(&mut a);
    *r = 7;
    a
}

pub fn generic_at_reference_vacuous(v: u32) -> u32 {
    let mut a = v;
    let r = generic_lender(&mut a);
    *r = 7;
    a
}

// What a callee cannot read at a type argument holding a mutable reference.

pub fn consume_reference(v: u32) -> u32 {
    let mut a = v;
    consume(&mut a);
    a
}

pub fn consume_value(v: u32) -> u32 {
    consume(v);
    v
}

pub fn consume_value_false(v: u32) -> u32 {
    consume(v);
    v
}

pub fn consume_value_vacuous(v: u32) -> u32 {
    consume(v);
    v
}

pub fn opaque_reference(v: u32) -> u32 {
    let mut a = v;
    let r = opaque_id(&mut a);
    *r = 7;
    a
}

pub fn opaque_value(v: u32) -> u32 {
    opaque_id(v)
}

pub fn opaque_value_false(v: u32) -> u32 {
    opaque_id(v)
}

pub fn opaque_value_vacuous(v: u32) -> u32 {
    opaque_id(v)
}

pub fn recursive_reference(v: u32) -> u32 {
    let mut a = v;
    let r = recursive_id(1, &mut a);
    *r = 7;
    a
}

pub fn native_reference(v: u32) -> u32 {
    let mut a = v;
    let mut items = Vec::new();
    items.push(&mut a);
    a
}

pub fn observed_reference(v: u32) -> u32 {
    let mut a = v;
    let r = observe(&mut a);
    *r = 7;
    a
}

pub fn observed_reference_false(v: u32) -> u32 {
    let mut a = v;
    let r = observe(&mut a);
    *r = 7;
    a
}

pub fn observed_reference_vacuous(v: u32) -> u32 {
    let mut a = v;
    let r = observe(&mut a);
    *r = 7;
    a
}

pub fn observed_value(v: u32) -> u32 {
    observe(v)
}

pub fn observed_value_false(v: u32) -> u32 {
    observe(v)
}

pub fn observed_value_vacuous(v: u32) -> u32 {
    observe(v)
}

pub fn viewing_value(v: u32) -> u32 {
    view_inside(v)
}

pub fn viewing_value_false(v: u32) -> u32 {
    view_inside(v)
}

pub fn viewing_value_vacuous(v: u32) -> u32 {
    view_inside(v)
}

pub fn nested_move(v: u32) -> u32 {
    let mut a = v;
    let w = outer(wrap(wrap(&mut a)));
    *w.inner = 7;
    a
}

// A concrete drop of a holder is not an instantiation: it dies through its paths.

pub struct DroppedHolder<'a> {
    pub r: &'a mut u32,
    pub owned: Vec<u32>,
}

pub fn holder_mem_drop(v: u32, owned: Vec<u32>) -> u32 {
    let mut a = v;
    let holder = DroppedHolder { r: &mut a, owned };
    *holder.r = 7;
    drop(holder);
    a
}

pub fn holder_dropped(v: u32, owned: Vec<u32>) -> u32 {
    let mut a = v;
    let holder = DroppedHolder { r: &mut a, owned };
    *holder.r = 7;
    let DroppedHolder { r: _, owned: _kept } = holder;
    a
}

pub fn holder_dropped_false(v: u32, owned: Vec<u32>) -> u32 {
    let mut a = v;
    let holder = DroppedHolder { r: &mut a, owned };
    *holder.r = 7;
    let DroppedHolder { r: _, owned: _kept } = holder;
    a
}

pub fn holder_scope_end(v: u32, owned: Vec<u32>) -> u32 {
    let mut a = v;
    {
        let holder = DroppedHolder { r: &mut a, owned };
        *holder.r = 7;
    }
    a
}

pub fn holder_scope_end_false(v: u32, owned: Vec<u32>) -> u32 {
    let mut a = v;
    {
        let holder = DroppedHolder { r: &mut a, owned };
        *holder.r = 7;
    }
    a
}

pub fn holder_scope_end_vacuous(v: u32, owned: Vec<u32>) -> u32 {
    let mut a = v;
    {
        let holder = DroppedHolder { r: &mut a, owned };
        *holder.r = 7;
    }
    a
}

pub fn push_view(v: u32) -> u32 {
    let mut a = v;
    let r = &mut a;
    let mut views = Vec::new();
    views.push(Viewing { view: &r });
    let n = views.len() as u32;
    *r = n;
    a
}
