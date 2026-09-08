pub trait Step {
    fn step(&mut self);
}

pub fn step_twice<T: Step>(value: &mut T) {
    value.step();
    value.step();
}
