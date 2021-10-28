address 0x1e {}
address /* '0x1' is 'Std' */ 0x1 {
    module ModuleOne {
        fun access_chains() {
            let i1 = 0xdde::Name::INTEGER;
            let i2 = 0xDDe/**/::/*.*/Name/*..*/::/*.*/INTEGER;
        }
    }
}
address Std /* 'Std' is '0x1' */ {
    module ModuleTwo { /* ... */ }
}

module Address::Module {}
