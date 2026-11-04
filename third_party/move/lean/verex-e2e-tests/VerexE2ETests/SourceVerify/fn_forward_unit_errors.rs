// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn unit_mut<F: FnMut()>(callback: &mut F) {
    callback();
}

pub fn unit_forward<F: Fn()>(callback: &mut F) {
    unit_mut(callback);
    unit_mut(callback);
}

pub fn omitted<F: Fn()>(callback: &mut F) {
    let _ = callback;
}
