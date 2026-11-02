/// Type tags for the aggregator's generic entry points, shaped after the
/// marker types Panora's composer threads through a route.
///
/// Nothing ever constructs one. They exist to be passed as type arguments and
/// read back with `type_info::type_of`, which is why they carry no fields and
/// no abilities: a flat tag costs one type node, so a thirty-two argument
/// instantiation stays far inside the verifier's hundred-and-twenty-eight node
/// budget.
module bench::dexr_markers {
    // Backend tags. `dexr_backend` dispatches on these.

    struct Cpmm {}
    struct Stable {}
    struct Clmm {}
    struct Book {}

    // Asset tags. `dexr_router::map_markers` binds these to real fungible
    // asset metadata; anything else resolves to the first asset.

    struct A0 {}
    struct A1 {}
    struct A2 {}
    struct A3 {}
    struct A4 {}
    struct A5 {}
    struct A6 {}
    struct A7 {}

    // Route, fee, and mode tags. Never mapped to an asset: they only widen
    // the instantiation and jitter which pool a hop lands on.

    struct M0 {}
    struct M1 {}
    struct M2 {}
    struct M3 {}
    struct M4 {}
    struct M5 {}
    struct M6 {}
    struct M7 {}
    struct M8 {}
    struct M9 {}
    struct M10 {}
    struct M11 {}
    struct M12 {}
    struct M13 {}
    struct M14 {}
    struct M15 {}
    struct M16 {}
    struct M17 {}
    struct M18 {}
    struct M19 {}
    struct M20 {}
    struct M21 {}
    struct M22 {}
    struct M23 {}
}
