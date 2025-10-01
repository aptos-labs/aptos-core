// Copyright Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0xABCD::locals {
    struct Data<A: copy + drop, B: copy + drop, C: copy + drop> has copy, drop {
        a: vector<A>,
        b: vector<vector<B>>,
        c: C,
        d: u128,
    }

    struct NestedData<
        A: copy + drop,
        B: copy + drop,
        C: copy + drop,
        D: copy + drop,
        E: copy + drop,
        F: copy + drop,
    > has copy, drop {
        abc: Data<A, B, C>,
        def: Data<D, E, F>,
    }

    fun work_with_many_locals() {
        let a = vector[0_u128];
        let b = vector[vector[0_u64]];
        let c = vector[0_u8];
        let d = 120;
        let data1 = Data {
            a,
            b,
            c,
            d,
        };
        let a = vector[0_u8];
        let b = vector[vector[120_u32]];
        let c = false;
        let d = 30;
        let data2 = Data {
            a,
            b,
            c,
            d,
        };
        let data3 = data1;
        let data4 = data2;
        let data5 = NestedData {
            abc: data3,
            def: data4,
        };
        let data6 = NestedData {
            abc: data4,
            def: data3,
        };
        let data7 = NestedData {
            abc: data4,
            def: data4,
        };
        let data8 = NestedData {
            abc: data3,
            def: data3,
        };
        assert!(data7.abc.d == data5.def.d, 777);
        assert!(data8.abc.d == data6.def.d, 888);
    }

    fun generic_work_with_many_locals<
        A: copy + drop,
        B: copy + drop,
        C: copy + drop,
        D: copy + drop,
        E: copy + drop,
        F: copy + drop,
    >() {
        let a = vector[];
        let b = vector[vector[]];
        let c = vector[false];
        let d = 40;
        let data1 = Data<A, B, vector<bool>> {
            a,
            b,
            c,
            d,
        };
        let a = vector[];
        let b = vector[vector[]];
        let c = data1;
        let d = 23;
        let data2 = Data<D, E, Data<A, B, vector<bool>>> {
            a,
            b,
            c,
            d,
        };
        let data3 = data1;
        let data4 = data2;
        let data5 = NestedData {
            abc: data3,
            def: data4,
        };
        let data6 = NestedData {
            abc: data4,
            def: data3,
        };
        let data7 = NestedData {
            abc: data4,
            def: data4,
        };
        let data8 = NestedData {
            abc: data3,
            def: data3,
        };
        assert!(data7.abc.d == data5.def.d, 777);
        assert!(data8.abc.d == data6.def.d, 888);
    }

    public entry fun benchmark() {
        let i = 0;
        while (i < 100) {
            generic_work_with_many_locals<u32, u32, bool, vector<bool>, Data<u8, u8, vector<bool>>, bool>();
            generic_work_with_many_locals<u32,  Data<u8, u32, vector<bool>>, bool, vector<bool>, Data<u8, u8, vector<bool>>, bool>();
            generic_work_with_many_locals<u32, u32, bool, Data<u8, u8, u32>, Data<u8, u8, vector<bool>>, bool>();
            generic_work_with_many_locals<u32, vector<bool>, Data<u8, u8, vector<bool>>, bool, u32, bool>();

            generic_work_with_many_locals<u32, address, bool, vector<bool>, Data<u8, u8, vector<bool>>, bool>();
            generic_work_with_many_locals<u32, u32, bool, vector<bool>, bool, Data<u8, u8, vector<bool>>>();
            generic_work_with_many_locals<Data<u8, u8, vector<bool>>, u32, u32, bool, vector<bool>, Data<u8, u8, vector<bool>>>();
            generic_work_with_many_locals<u32, Data<u8, u8, vector<bool>>, bool, vector<bool>, Data<u8, u8, vector<bool>>, bool>();

            generic_work_with_many_locals<u32, u32, Data<Data<u8, u8, vector<bool>>, u8, vector<bool>>, vector<bool>, Data<u8, u8, vector<bool>>, bool>();
            generic_work_with_many_locals<u32, Data<Data<u8, u8, vector<bool>>, Data<u8, u8, vector<bool>>, vector<bool>>, bool, vector<bool>, Data<u8, u8, vector<bool>>, bool>();
            generic_work_with_many_locals<u32, u32, bool, vector<bool>, Data<u8, u8, vector<bool>>, bool>();
            generic_work_with_many_locals<u32, u32, Data<bool, bool, bool>, vector<bool>, Data<u8, u8, vector<bool>>, bool>();

            generic_work_with_many_locals<u32, Data<Data<Data<bool, bool, bool>, bool, bool>, bool, bool>, bool, vector<bool>, Data<u8, u8, vector<bool>>, bool>();
            generic_work_with_many_locals<u32, u32, bool, vector<bool>, Data<u8, u8, vector<Data<bool, bool, bool>>>, bool>();
            generic_work_with_many_locals<u32, u32, bool, vector<Data<u8, Data<bool, Data<bool, bool, bool>, bool>, u32>>, Data<u8, u8, vector<bool>>, bool>();
            generic_work_with_many_locals<u32, NestedData<u8, u8, u8, u32, u8, vector<vector<vector<bool>>>>, bool, vector<bool>, Data<u8, u8, vector<Data<bool, bool, bool>>>, Data<bool, bool, bool>>();
            i = i + 1;
        }
    }

    public entry fun benchmark_generic() {
        let i = 0;
        while (i < 100) {
            work_with_many_locals();
            work_with_many_locals();
            work_with_many_locals();
            work_with_many_locals();

            work_with_many_locals();
            work_with_many_locals();
            work_with_many_locals();
            work_with_many_locals();

            work_with_many_locals();
            work_with_many_locals();
            work_with_many_locals();
            work_with_many_locals();

            work_with_many_locals();
            work_with_many_locals();
            work_with_many_locals();
            work_with_many_locals();
            i = i + 1;
        }
    }
}
