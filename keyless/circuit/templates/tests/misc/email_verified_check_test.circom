
pragma circom 2.1.3;

include "helpers/misc.circom";

template email_verified_check_test() {
    var maxEVNameLen = 20;
    var maxEVValueLen = 10;
    var maxUIDNameLen = 30;
    signal input ev_name[maxEVNameLen];
    signal input ev_value[maxEVValueLen];
    signal input ev_value_len;
    signal input uid_name[maxUIDNameLen];
    signal input uid_name_len;
    signal input uid_is_email;
    component email_verified_check = EmailVerifiedCheck(maxEVNameLen, maxEVValueLen, maxUIDNameLen);
    email_verified_check.ev_name <== ev_name;
    email_verified_check.ev_value <== ev_value;
    email_verified_check.ev_value_len <== ev_value_len;
    email_verified_check.uid_name <== uid_name;
    email_verified_check.uid_name_len <== uid_name_len;
    email_verified_check.uid_is_email === uid_is_email;

}

component main = email_verified_check_test(
);


