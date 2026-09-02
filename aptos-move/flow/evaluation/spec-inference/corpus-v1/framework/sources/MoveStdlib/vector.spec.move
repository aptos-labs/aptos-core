spec std::vector {

    spec range(start: u64, end: u64): vector<u64> {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] result == result_of<range_with_step>(start, end, 1);
    }

    spec replace<Element>(self: &mut vector<Element>, i: u64, val: Element): Element {
        use 0x1::mem;
        pragma opaque = true;
        ensures [inferred] i < len(old(self)) ==>
            result == result_of<mem::replace<Element>> (old(self)[i], val);
        ensures [inferred] i < len(old(self)) ==> self == old(self);
        ensures [inferred] i >= len(old(self)) ==> self == old(self);
        aborts_if [inferred] i < len(self) && !in_range(self, i);
        aborts_if [inferred] i >= len(self);
    }

    spec last<Element>(self: &vector<Element>): &Element {
        pragma opaque = true;
        ensures [inferred] len(self) > 0 ==> result == self[len(self) - 1];
        aborts_if [inferred] len(self) > 0 && !in_range(self, len(self) - 1);
        aborts_if [inferred] len(self) == 0;
    }

    spec last_mut<Element>(self: &mut vector<Element>): &mut Element {
        pragma opaque = true;
        ensures [inferred] len(old(self)) > 0 ==>
            result == old(self)[len(old(self)) - 1];
        ensures [inferred] len(old(self)) > 0 ==> self == old(self);
        ensures [inferred] len(old(self)) == 0 ==> self == old(self);
        aborts_if [inferred] len(self) > 0 && !in_range(self, len(self) - 1);
        aborts_if [inferred] len(self) == 0;
    }

    spec range_with_step(start: u64, end: u64, step: u64): vector<u64> {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = sathard] step > 0 ==>
            (forall x: vector<u64> : result == x);
        aborts_if [inferred = sathard] step == 0;
        aborts_if [inferred = sathard] end + step > 18446744073709551616;
    }

    spec singleton<Element>(e: Element): vector<Element> {
        pragma opaque = true;
        ensures [inferred] result == vec(e);
        aborts_if [inferred] false;
    }

    spec slice<Element: copy>(self: &vector<Element>, start: u64, end: u64)
        : vector<Element> {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = sathard] start <= end && end <= len(self) ==>
            (forall x: vector<Element> : result == x);
        aborts_if [inferred = sathard] start <= end && end > len(self);
        aborts_if [inferred = sathard] start <= end
            && (end <= len(self) && end == 18446744073709551616);
        aborts_if [inferred = sathard] start <= end
            && (end <= len(self)
                && (exists x: u64: x < end
                    && !in_range(self, x)));
        aborts_if [inferred = sathard] start > end;
    }
}
