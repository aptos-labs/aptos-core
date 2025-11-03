//# publish
module 0xCAFE::Module0 {
    enum Enum0 has copy, drop {
        Variant0,
        Variant1 {
            field: bool,
        },
        Variant2,
    }
    public fun function3( var0: bool, var1: bool) {
        match (Enum0::Variant0) {
            Enum0::Variant2 => {
                1
            },
            Enum0::Variant1 {..} => {
                    let var28 = 1;
                     (!var1) && var0;
                    var28
            },
            Enum0::Variant0 => {
                1
            }
        };
    }
}
