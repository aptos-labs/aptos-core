//# init --addresses    Alice=0x1ca48626b4a6af9ee9247b9c87c56622
//#                     Bob=0x6333024799dc0e04f6abb6d9b5cb48ec
//#                     Child=0x1987fd30165c76ee3f46dc50f8932ce5
//#      --private-keys Alice=6184cda1a6d69519bc2d911da5aa61b3c0a021debf52291e0f632b39be577f64
//#                     Bob=e0f70dd3b1870976bac3784ca02745826f548f50315b4f107d52c7e041665946
//#                     Child=c2596670129d3ab8b5daf78b58374471fddea19030a912a2d3c9a3f55cc61a16


//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0
//#            @Bob
//#            x"4ef8ded2db202bcde9c070fff7be48bc"
//#            b"bob"
//#            true
//#     -- 0x1::AccountCreationScripts::create_parent_vasp_account


//# run --signers Bob
//#     --type-args 0x1::XUS::XUS
//#     --args @Child
//#            x"853ca357812881559c72de8350a460fb"
//#            true
//#            0
//#     -- 0x1::AccountCreationScripts::create_child_vasp_account


//# run --signers TreasuryCompliance
//#     --type-args 0x1::XDX::XDX
//#     --args 0
//#            @Alice
//#            x"0fb58139cf145f855e50686d787185bd"
//#            b"alice"
//#            true
//#     -- 0x1::AccountCreationScripts::create_parent_vasp_account

// TODO: is this a duplicate of vasps.move?
