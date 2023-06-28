
module 0x100::Country {
    struct Dunno has drop, copy { x: u64 }
    struct Country has drop, copy {
        id: u8,
        population: u64,
        phony: Dunno
    }
    public fun new_country(i: u8, p: u64): Country {
        Country { id: i, population: p, phony: Dunno { x: 32 } }
    }
    public fun get_pop(cc: Country): u64 {
        cc.population
    }
    public fun get_phony_x(cc: Country): u64 {
        cc.phony.x
    }
    public fun get_id(cc: &Country): u8 {
        cc.id
    }
    public fun set_id(cc: &mut Country, t: u8) {
        cc.id = t;
    }
    public fun dropit(cc: Country): u8 {
        let Country { id: x, population: _y, phony: _z } = cc;
        x
    }
}

module 0x200::UseIt {
    struct NotUsed {}
    use 0x100::Country;

    public fun getit() {
        let c = Country::new_country(1, 1000000);
        let c2 = c;
        Country::get_pop(c2);
        Country::get_id(&c2);
        Country::set_id(&mut c2, 123);
        Country::dropit(c2);
    }
}
