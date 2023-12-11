module NamedAddr::Detector {
    use std::vector;

    public fun deep_nesting_check(addr: address, reward_token: u64) {
        let new_reward = vector::empty<u64>();
        let current_amount = vector::borrow_mut(&mut new_reward, reward_token);
        if (2 == 3){
            *current_amount = *current_amount + 3;
            if(reward_token == 3){
                *current_amount = *current_amount + 3;
                
                if(reward_token == 3){
                    let current_amount2 = vector::borrow_mut(&mut new_reward, reward_token);
                    *current_amount2 = *current_amount2 + 3;
                    if(reward_token == 4){
                        let current_amount2 = vector::borrow_mut(&mut new_reward, reward_token);
                        *current_amount2 = *current_amount2 + 3;
                        if(reward_token == 5){
                            let current_amount2 = vector::borrow_mut(&mut new_reward, reward_token);
                            *current_amount2 = *current_amount2 + 3;
                            if(reward_token == 6){
                                let current_amount2 = vector::borrow_mut(&mut new_reward, reward_token);
                                *current_amount2 = *current_amount2 + 3;
                            }
                        }
                    }
                }
            }
        };

        if(reward_token == 8) {
            let a = 2;
            if(reward_token == 9){
                a = 1;
                if(reward_token == 10){
                    a = 1;
                    if(reward_token == 11){
                        a = 1;
                        if(reward_token == 12){
                            a = 2;
                            if(reward_token == 13){
                                a = 3;
                                
                            }
                        }
                    }
                }
            }
        };
    }
}