// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn opt_by_ref(o: &mut Option<u8>, v: u8) {
    if let Some(r) = o {
        *r = v;
    }
}

pub fn opt_by_ref_false(o: &mut Option<u8>, v: u8) {
    if let Some(r) = o {
        *r = v;
    }
}

pub fn opt_by_ref_vacuous(o: &mut Option<u8>, v: u8) {
    if let Some(r) = o {
        *r = v;
    }
}

pub fn opt_ref_of_ref(o: &mut Option<&mut u8>, v: u8) {
    if let Some(r) = o {
        **r = v;
    }
}

pub fn opt_ref_of_ref_false(o: &mut Option<&mut u8>, v: u8) {
    if let Some(r) = o {
        **r = v;
    }
}

pub fn opt_ref_of_ref_vacuous(o: &mut Option<&mut u8>, v: u8) {
    if let Some(r) = o {
        **r = v;
    }
}

pub fn opt_none_on_one_path(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let mut o = Some(&mut x);
    if c {
        o = None;
    }
    if let Some(r) = o {
        *r = v;
    }
    x
}

pub fn opt_none_on_one_path_false(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let mut o = Some(&mut x);
    if c {
        o = None;
    }
    if let Some(r) = o {
        *r = v;
    }
    x
}

pub fn opt_none_on_one_path_vacuous(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let mut o = Some(&mut x);
    if c {
        o = None;
    }
    if let Some(r) = o {
        *r = v;
    }
    x
}
