
pragma circom 2.1.3;

include "helpers/misc.circom";

template calculate_total_test() {
    var len = 10;
    signal input nums[len];
    signal input sum;
    component calculate_total = CalculateTotal(len);
    calculate_total.nums <== nums;
    calculate_total.sum === sum;

}

component main = calculate_total_test(
);
