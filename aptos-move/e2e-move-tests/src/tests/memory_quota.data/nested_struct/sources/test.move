module publisher::very_nested_structure {
    use aptos_std::table;
    use aptos_std::table::Table;
    use std::vector;
    use std::error;
    use std::signer;
    use std::debug;

    struct W0 has copy, drop, store {}
    struct W1 has copy, drop, store { id: W0 }
    struct W2 has copy, drop, store { id: W1 }
    struct W3 has copy, drop, store { id: W2 }
    struct W4 has copy, drop, store { id: W3 }
    struct W5 has copy, drop, store { id: W4 }
    struct W6 has copy, drop, store { id: W5 }
    struct W7 has copy, drop, store { id: W6 }
    struct W8 has copy, drop, store { id: W7 }
    struct W9 has copy, drop, store { id: W8 }
    struct W10 has copy, drop, store { id: W9 }
    struct W11 has copy, drop, store { id: W10 }
    struct W12 has copy, drop, store { id: W11 }
    struct W13 has copy, drop, store { id: W12 }
    struct W14 has copy, drop, store { id: W13 }
    struct W15 has copy, drop, store { id: W14 }
    struct W16 has copy, drop, store { id: W15 }
    struct W17 has copy, drop, store { id: W16 }
    struct W18 has copy, drop, store { id: W17 }
    struct W19 has copy, drop, store { id: W18 }
    struct W20 has copy, drop, store { id: W19 }
    struct W21 has copy, drop, store { id: W20 }
    struct W22 has copy, drop, store { id: W21 }
    struct W23 has copy, drop, store { id: W22 }
    struct W24 has copy, drop, store { id: W23 }
    struct W25 has copy, drop, store { id: W24 }
    struct W26 has copy, drop, store { id: W25 }
    struct W27 has copy, drop, store { id: W26 }
    struct W28 has copy, drop, store { id: W27 }
    struct W29 has copy, drop, store { id: W28 }
    struct W30 has copy, drop, store { id: W29 }
    struct W31 has copy, drop, store { id: W30 }
    struct W32 has copy, drop, store { id: W31 }
    struct W33 has copy, drop, store { id: W32 }
    struct W34 has copy, drop, store { id: W33 }
    struct W35 has copy, drop, store { id: W34 }
    struct W36 has copy, drop, store { id: W35 }
    struct W37 has copy, drop, store { id: W36 }
    struct W38 has copy, drop, store { id: W37 }
    struct W39 has copy, drop, store { id: W38 }
    struct W40 has copy, drop, store { id: W39 }
    struct W41 has copy, drop, store { id: W40 }
    struct W42 has copy, drop, store { id: W41 }
    struct W43 has copy, drop, store { id: W42 }
    struct W44 has copy, drop, store { id: W43 }
    struct W45 has copy, drop, store { id: W44 }
    struct W46 has copy, drop, store { id: W45 }
    struct W47 has copy, drop, store { id: W46 }
    struct W48 has copy, drop, store { id: W47 }
    struct W49 has copy, drop, store { id: W48 }
    struct W50 has copy, drop, store { id: W49 }
    struct W51 has copy, drop, store { id: W50 }
    struct W52 has copy, drop, store { id: W51 }
    struct W53 has copy, drop, store { id: W52 }
    struct W54 has copy, drop, store { id: W53 }
    struct W55 has copy, drop, store { id: W54 }
    struct W56 has copy, drop, store { id: W55 }
    struct W57 has copy, drop, store { id: W56 }
    struct W58 has copy, drop, store { id: W57 }
    struct W59 has copy, drop, store { id: W58 }
    struct W60 has copy, drop, store { id: W59 }
    struct W61 has copy, drop, store { id: W60 }
    struct W62 has copy, drop, store { id: W61 }
    struct W63 has copy, drop, store { id: W62 }
    struct W64 has copy, drop, store { id: W63 }
    struct W65 has copy, drop, store { id: W64 }
    struct W66 has copy, drop, store { id: W65 }
    struct W67 has copy, drop, store { id: W66 }
    struct W68 has copy, drop, store { id: W67 }
    struct W69 has copy, drop, store { id: W68 }
    struct W70 has copy, drop, store { id: W69 }
    struct W71 has copy, drop, store { id: W70 }
    struct W72 has copy, drop, store { id: W71 }
    struct W73 has copy, drop, store { id: W72 }
    struct W74 has copy, drop, store { id: W73 }
    struct W75 has copy, drop, store { id: W74 }
    struct W76 has copy, drop, store { id: W75 }
    struct W77 has copy, drop, store { id: W76 }
    struct W78 has copy, drop, store { id: W77 }
    struct W79 has copy, drop, store { id: W78 }
    struct W80 has copy, drop, store { id: W79 }
    struct W81 has copy, drop, store { id: W80 }
    struct W82 has copy, drop, store { id: W81 }
    struct W83 has copy, drop, store { id: W82 }
    struct W84 has copy, drop, store { id: W83 }
    struct W85 has copy, drop, store { id: W84 }
    struct W86 has copy, drop, store { id: W85 }
    struct W87 has copy, drop, store { id: W86 }
    struct W88 has copy, drop, store { id: W87 }
    struct W89 has copy, drop, store { id: W88 }
    struct W90 has copy, drop, store { id: W89 }
    struct W91 has copy, drop, store { id: W90 }
    struct W92 has copy, drop, store { id: W91 }
    struct W93 has copy, drop, store { id: W92 }
    struct W94 has copy, drop, store { id: W93 }
    struct W95 has copy, drop, store { id: W94 }
    struct W96 has copy, drop, store { id: W95 }
    struct W97 has copy, drop, store { id: W96 }
    struct W98 has copy, drop, store { id: W97 }
    struct W99 has copy, drop, store { id: W98 }

    public struct NestedStructVector has key, drop, store {
        wraps: vector<W99>,
    }

    struct GlobalHolder has key, store {
        table: Table<u64, NestedStructVector>,
        counter: u64,
    }

    public entry fun init(payer: &signer) {
        if (!exists<GlobalHolder>(signer::address_of(payer))) {
            let tbl : Table<u64, NestedStructVector> = table::new();
            let holder = GlobalHolder { table: tbl, counter: 0 };
            move_to(payer, holder);
        }
    }

    fun create_w99(): W99 {
        let w0 = W0 {};
        let w1 = W1 { id: w0 };
        let w2 = W2 { id: w1 };
        let w3 = W3 { id: w2 };
        let w4 = W4 { id: w3 };
        let w5 = W5 { id: w4 };
        let w6 = W6 { id: w5 };
        let w7 = W7 { id: w6 };
        let w8 = W8 { id: w7 };
        let w9 = W9 { id: w8 };
        let w10 = W10 { id: w9 };
        let w11 = W11 { id: w10 };
        let w12 = W12 { id: w11 };
        let w13 = W13 { id: w12 };
        let w14 = W14 { id: w13 };
        let w15 = W15 { id: w14 };
        let w16 = W16 { id: w15 };
        let w17 = W17 { id: w16 };
        let w18 = W18 { id: w17 };
        let w19 = W19 { id: w18 };
        let w20 = W20 { id: w19 };
        let w21 = W21 { id: w20 };
        let w22 = W22 { id: w21 };
        let w23 = W23 { id: w22 };
        let w24 = W24 { id: w23 };
        let w25 = W25 { id: w24 };
        let w26 = W26 { id: w25 };
        let w27 = W27 { id: w26 };
        let w28 = W28 { id: w27 };
        let w29 = W29 { id: w28 };
        let w30 = W30 { id: w29 };
        let w31 = W31 { id: w30 };
        let w32 = W32 { id: w31 };
        let w33 = W33 { id: w32 };
        let w34 = W34 { id: w33 };
        let w35 = W35 { id: w34 };
        let w36 = W36 { id: w35 };
        let w37 = W37 { id: w36 };
        let w38 = W38 { id: w37 };
        let w39 = W39 { id: w38 };
        let w40 = W40 { id: w39 };
        let w41 = W41 { id: w40 };
        let w42 = W42 { id: w41 };
        let w43 = W43 { id: w42 };
        let w44 = W44 { id: w43 };
        let w45 = W45 { id: w44 };
        let w46 = W46 { id: w45 };
        let w47 = W47 { id: w46 };
        let w48 = W48 { id: w47 };
        let w49 = W49 { id: w48 };
        let w50 = W50 { id: w49 };
        let w51 = W51 { id: w50 };
        let w52 = W52 { id: w51 };
        let w53 = W53 { id: w52 };
        let w54 = W54 { id: w53 };
        let w55 = W55 { id: w54 };
        let w56 = W56 { id: w55 };
        let w57 = W57 { id: w56 };
        let w58 = W58 { id: w57 };
        let w59 = W59 { id: w58 };
        let w60 = W60 { id: w59 };
        let w61 = W61 { id: w60 };
        let w62 = W62 { id: w61 };
        let w63 = W63 { id: w62 };
        let w64 = W64 { id: w63 };
        let w65 = W65 { id: w64 };
        let w66 = W66 { id: w65 };
        let w67 = W67 { id: w66 };
        let w68 = W68 { id: w67 };
        let w69 = W69 { id: w68 };
        let w70 = W70 { id: w69 };
        let w71 = W71 { id: w70 };
        let w72 = W72 { id: w71 };
        let w73 = W73 { id: w72 };
        let w74 = W74 { id: w73 };
        let w75 = W75 { id: w74 };
        let w76 = W76 { id: w75 };
        let w77 = W77 { id: w76 };
        let w78 = W78 { id: w77 };
        let w79 = W79 { id: w78 };
        let w80 = W80 { id: w79 };
        let w81 = W81 { id: w80 };
        let w82 = W82 { id: w81 };
        let w83 = W83 { id: w82 };
        let w84 = W84 { id: w83 };
        let w85 = W85 { id: w84 };
        let w86 = W86 { id: w85 };
        let w87 = W87 { id: w86 };
        let w88 = W88 { id: w87 };
        let w89 = W89 { id: w88 };
        let w90 = W90 { id: w89 };
        let w91 = W91 { id: w90 };
        let w92 = W92 { id: w91 };
        let w93 = W93 { id: w92 };
        let w94 = W94 { id: w93 };
        let w95 = W95 { id: w94 };
        let w96 = W96 { id: w95 };
        let w97 = W97 { id: w96 };
        let w98 = W98 { id: w97 };
        W99 { id: w98 }
    }

    public entry fun add(account: &signer, length: u64) acquires GlobalHolder {
        assert!(exists<GlobalHolder>(signer::address_of(account)), error::invalid_state(0));

        let w99 = create_w99();
        let wraps = vector::empty<W99>();

        let i = 0;
        while (i < length) {
            wraps.push_back(w99);
            i = i + 1;
        };

        let t = NestedStructVector { wraps };

        let holder = borrow_global_mut<GlobalHolder>(signer::address_of(account));
        table::add(&mut holder.table, holder.counter, t);
        holder.counter = holder.counter + 1;
    }

    public entry fun read_all(
        account: &signer,
    ) acquires GlobalHolder {
        let holder = borrow_global<GlobalHolder>(signer::address_of(account));

        let current_id = 0;
        let sum = 0;
        while (current_id < holder.counter) {
            sum = sum + vector::length(&table::borrow(&holder.table, current_id).wraps);
            current_id = current_id + 1;
        };
    }

}
