module NamedAddr::counter {
    use aptos_std::table::{Self, Table};
    use aptos_std::simple_map::{Self, SimpleMap};
    use aptos_framework::object::{Self, Object};
    use std::vector;
    struct Counter has key { i: u64, z: u64 }

    // struct CounterList has key {
    //     list: Table<u64, Counter>,
    // }

    // struct Rewards has store {
    //     total_amounts: SimpleMap<Object<Counter>, u64>,
    // }

    public fun borrow_mutate_and_call_functions(addr: address) acquires Counter {
        let c_ref = borrow_global_mut<Counter>(addr);
        passed_by_mutable(c_ref, 0);
        let d_ref = borrow_global_mut<Counter>(addr);
        passed_by_immutable(d_ref, 0);
    }

    public fun passed_by_immutable(coin: &mut Counter, amount: u64) : u64  {
        amount
    }

    public fun passed_by_mutable(coin: &Counter, amount: u64) : u64  {
        amount
    }

    public fun scope_nested_test(addr: address) acquires Counter {
        let c_ref = &mut borrow_global_mut<Counter>(addr).i;
        let d_ref = &mut borrow_global_mut<Counter>(addr).i;
        let c_ref = 1;
        c_ref = c_ref + 5;
        c_ref = c_ref + 4;
        c_ref = c_ref + 2;
        if(3 == 2){
            let a = 2;
            let b = 3;
            if (3 == 1){
                let d = 3;
                let c = 3;
                c = c+1;
                *c_ref = 2;
                let c_ref = &mut borrow_global_mut<Counter>(addr).i;
                let e_ref = &mut borrow_global_mut<Counter>(addr).i;
                let f_ref = &mut borrow_global_mut<Counter>(addr).i;
                let g_ref = &mut borrow_global_mut<Counter>(addr).i;
                *g_ref = 2;
            }
        }

    }

    public fun scope_nearest_nested_test(addr: address) acquires Counter {
        let c_ref = &mut borrow_global_mut<Counter>(addr).i;
        if (2 == 3){
            let alo = 2;
            if(false){
                let blo = 2;
                if(true){
                    let c_ref = &mut borrow_global_mut<Counter>(addr).i;
                    *c_ref = 2;
                }

            }
        }
    }

    public fun loop_test(addr: address) acquires Counter {
        let c_ref = &mut borrow_global_mut<Counter>(addr);
        let d_ref = &mut borrow_global_mut<Counter>(addr).i;
        let f_ref = &mut borrow_global_mut<Counter>(addr).i;
        loop {
            if (1 == 1) {
                
            }
        };
        *c_ref = 2;
    }

    public entry fun table_borrow_mutable_test(addr: address, task_id: u64) acquires CounterList {
        let todo_list = borrow_global_mut<CounterList>(addr);
        let task_record = table::borrow_mut(&mut todo_list.list, task_id);
        if (2 == 3){
                if(true){
                    task_record.i = 2;
                    let task_record2 = table::borrow_mut(&mut todo_list.list, task_id);
                    let task_record3 = table::borrow_mut(&mut todo_list.list, task_id);
                    task_record3.i = 2;
                }

        }
    }
    
    public entry fun simple_map_borrow_mutable_test(addr: address, reward_token: u64) {
        let new_reward = simple_map::new<u64, u64>();
        let current_amount = simple_map::borrow_mut(&mut new_reward, &reward_token);
        if (2 == 3){
            if(true){
                *current_amount = *current_amount + 3;
                let simple_map1 = simple_map::borrow_mut(&mut new_reward, &reward_token);
                let simple_map2 = simple_map::borrow_mut(&mut new_reward, &reward_token);
                *simple_map2 = *simple_map2 + 3;
            }
        }
    }

    public entry fun vector_borrow_mutable_test(addr: address, reward_token: u64) {
        let new_reward = vector::empty<u64>();
        let current_amount = vector::borrow_mut(&mut new_reward, reward_token);
        if (2 == 3){
            *current_amount = *current_amount + 3;
            if(true){
                let current_amount1 = vector::borrow_mut(&mut new_reward, reward_token);
                let current_amount2 = vector::borrow_mut(&mut new_reward, reward_token);
                *current_amount2 = *current_amount2 + 3;
            }
        }
    }

}