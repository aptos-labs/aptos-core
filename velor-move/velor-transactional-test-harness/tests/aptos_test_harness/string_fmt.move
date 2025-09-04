//# init --addresses Alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#   --private-keys Alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f
//#   --initial-coins 1000000000000

//# publish --private-key Alice
module Alice::StringFmt {
    use std::string_utils;
    use std::string;

    public entry fun test() {
        let ss = string::utf8(b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        let i = 0;
        // it creates 2x larger string each time
        while (i < 13) {
            ss = string_utils::debug_string(ss.bytes());
            i = i + 1;
        };
        loop {
            string_utils::debug_string(ss.bytes());
        };
    }
}

//# run --signers Alice --show-events -- Alice::StringFmt::test
