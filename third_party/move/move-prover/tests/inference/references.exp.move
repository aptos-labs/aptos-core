// Test spec inference for reference operations
module 0x42::references {

    struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    enum Color has copy, drop {
        Red,
        RGB { r: u8, g: u8, b: u8 },
    }

    // BorrowLoc + ReadRef - should infer: ensures result == x
    fun read_local(x: u64): u64 {
        let r = &x;
        *r
    }
    spec read_local(x: u64): u64 {
        ensures [inferred] result == x;
    }


    // BorrowField + ReadRef - should infer: ensures result == p.x
    fun read_field(p: Point): u64 {
        let r = &p.x;
        *r
    }
    spec read_field(p: Point): u64 {
        ensures [inferred] result == p.x;
    }


    // Multiple field reads - should infer: ensures result == p.x + p.y
    fun sum_fields(p: Point): u64 {
        let rx = &p.x;
        let ry = &p.y;
        *rx + *ry
    }
    spec sum_fields(p: Point): u64 {
        ensures [inferred] result == p.x + p.y;
        aborts_if [inferred] p.x + p.y > MAX_U64;
    }


    // Nested borrow (borrow field of borrowed struct)
    fun nested_read(p: Point): u64 {
        let rp = &p;
        let rx = &rp.x;
        *rx
    }
    spec nested_read(p: Point): u64 {
        ensures [inferred] result == p.x;
    }


    // FreezeRef - convert mutable to immutable reference
    fun freeze_and_read(p: &mut Point): u64 {
        let rp = freeze(p);
        let rx = &rp.x;
        *rx
    }
    spec freeze_and_read(p: &mut Point): u64 {
        ensures [inferred] result == p.x;
        ensures [inferred] p == old(p);
    }


    // Pattern match variants with references.
    fun read_rgb_red(c: Color): u8 {
        match (&c) {
            Color::RGB { r, g: _, b: _ } => *r,
            Color::Red => 0,
        }
    }
    spec read_rgb_red(c: Color): u8 {
        ensures [inferred] (c is RGB) ==> result == c.r;
        ensures [inferred] !(c is RGB) ==> result == 0;
    }


    // Simple mutable reference read
    fun read_mut_field(p: &mut Point): u64 {
        let rx = &p.x;
        *rx
    }
    spec read_mut_field(p: &mut Point): u64 {
        ensures [inferred] result == p.x;
        ensures [inferred] p == old(p);
    }


    // Direct variant field access - aborts if not RGB variant
    // Should infer: ensures result == c.r, aborts_if !is_RGB(c)
    fun get_red_component(c: Color): u8 {
        c.r
    }
    spec get_red_component(c: Color): u8 {
        ensures [inferred] result == c.r;
        aborts_if [inferred] !(c is RGB);
    }


    // Variant field borrow through reference - aborts if not RGB
    // Should infer: ensures result == c.g, aborts_if !is_RGB(c)
    fun get_green_via_ref(c: &Color): u8 {
        c.g
    }
    spec get_green_via_ref(c: &Color): u8 {
        ensures [inferred] result == c.g;
        aborts_if [inferred] !(c is RGB);
    }


    // Multiple variant field reads - aborts if not RGB
    fun sum_rgb(c: Color): u16 {
        (c.r as u16) + (c.g as u16) + (c.b as u16)
    }
    spec sum_rgb(c: Color): u16 {
        ensures [inferred] result == (c.r as u16) + (c.g as u16) + (c.b as u16);
        aborts_if [inferred] (c.r as u16) + (c.g as u16) + (c.b as u16) > MAX_U16;
        aborts_if [inferred] !(c is RGB);
        aborts_if [inferred] (c.r as u16) + (c.g as u16) > MAX_U16;
    }

}
/*
Verification: Succeeded.
*/
