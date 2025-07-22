module 0x99::enum_simple {
    // Note: `Shape` has no `drop` ability, so must be destroyed with explicit unpacking.
    enum Shape {
        Circle{radius: u64},
        Rectangle{width: u64, height: u64}
    }

    fun destroy_empty(self: Shape) : bool {
        match (self) {
            Shape::Circle{radius} => true,
            Shape::Rectangle{width, height: _} => false,
        }
    }

    fun example_destroy_shapes() {
        let c = Shape::Circle{radius: 0};
        let r = Shape::Rectangle{width: 0, height: 0};
        c.destroy_empty();
        r.destroy_empty();
    }
}



module 0x99::enum_complex {
    enum Admin has drop {
        Superuser,
        User {
            _0: u64,
        }
    }
    enum Entity has drop {
        Person {
            id: u64,
        }
        Institution {
            id: u64,
            admin: Admin,
        }
    }
    fun admin_id(self: &Entity): u64 {
        let _t8;
        let _t6;
        let _t3;
        let _t1;
        let _t2;
        let _t5;
        'l0: loop {
            if (!((self is Institution) && (&self.admin is Superuser))) {
                loop {
                    if (self is Institution) {
                        _t5 = &self.admin;
                        if (_t5 is User) {
                            _t2 = &_t5._0;
                            if (*_t2 > 10) break
                        }
                    };
                    loop {
                        if (self is Institution) {
                            _t5 = &self.admin;
                            if (_t5 is User) {
                                if (*&_t5._0 <= 10) break}
                        };
                        _t1 = self;
                        if (_t1 is Person) {
                            _t2 = &_t1.id;
                            if (*_t2 > 10) {
                                _t3 = *_t2;
                                break 'l0
                            }
                        };
                        if (_t1 is Institution) {
                            _t3 = *&_t1.id;
                            break 'l0
                        };
                        _t3 = 0;
                        break 'l0
                    };
                    _t1 = self;
                    loop {
                        if (_t1 is Person) {
                            _t2 = &_t1.id;
                            if (*_t2 > 10) {
                                _t6 = *_t2;
                                break
                            }
                        };
                        if (_t1 is Institution) {
                            _t6 = *&_t1.id;
                            break
                        };
                        _t6 = 0;
                        break
                    };
                    _t3 = _t6 + 5;
                    break 'l0
                };
                _t1 = self;
                loop {
                    if (_t1 is Person) {
                        _t2 = &_t1.id;
                        if (*_t2 > 10) {
                            _t6 = *_t2;
                            break
                        }
                    };
                    if (_t1 is Institution) {
                        _t6 = *&_t1.id;
                        break
                    };
                    _t6 = 0;
                    break
                };
                _t3 = *_t2 + _t6;
                break
            };
            loop {
                if (self is Person) {
                    _t2 = &self.id;
                    if (*_t2 > 10) {
                        _t8 = *_t2;
                        break
                    }
                };
                if (self is Institution) {
                    _t8 = *&self.id;
                    break
                };
                _t8 = 0;
                break
            };
            _t3 = 1 + _t8;
            break
        };
        _t3
    }
}
