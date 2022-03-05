//# init --parent-vasps Alice --validators Vivian

//# block --proposer Vivian --time 100000000

//# publish --expiration 99
module Alice::M1 {}

//# publish --expiration 100
module Alice::M2 {}

//# publish --expiration 101
module Alice::M3 {}

//# publish --expiration 86500
module Alice::M4 {}

//# block --proposer Vivian --time 101000000

//# publish --expiration 86500
module Alice::M5 {}

//# publish --expiration 101
module Alice::M6 {}

//# publish --expiration 18446744073710
module Alice::M7 {}
