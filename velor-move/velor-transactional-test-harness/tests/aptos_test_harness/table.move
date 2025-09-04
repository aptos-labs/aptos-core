//# init --addresses Alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6 Bob=0x9c3b634ac05d0af393e0f93b9b19b61e7cac1c519f566276aa0c6fd15dac12aa
//#      --private-keys Alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f Bob=952aaf3a98a27903dd078d76fc9e411740d2ae9dd9ecb87b96c7cd6b791ffc69
//#      --initial-coins 100000000


//# run --signers Alice --args x"6170746f735f70756e6b73" x"" x"" 100 x"000000" --show-events -- 0x3::token::create_collection_script

// Mint "geek_token" for collection "velor_punks"
//# run --signers Alice --args x"6170746f735f70756e6b73" x"6765656b5f746f6b656e" x"" 10 100 x"" @Alice 1 0 x"0000000000" x"" x"" x"" --show-events -- 0x3::token::create_token_script

//# view --address Alice  --resource 0x3::token::Collections

//# view_table --table_handle 0x5adf7712b6186bb577ebd9e2067ef3d46014f5bb5fa1c457f51a334295affe9a --key_type 0x1::string::String --value_type 0x3::token::CollectionData --key_value "velor_punks"

// Alice offers tokens to Bob
//# run --signers Alice --args @Bob @Alice x"6170746f735f70756e6b73" x"6765656b5f746f6b656e" 0 1 --show-events -- 0x3::token_transfers::offer_script

// Bob accepts tokens from Alice
//# run --signers Bob --args @Alice @Alice x"6170746f735f70756e6b73" x"6765656b5f746f6b656e" 0 --show-events -- 0x3::token_transfers::claim_script

//# view_table --table_handle 0x630c3a2b1a1f2a5a9d112eb98bfe1b35265a16f1482e6224950abd8ad4c76d52 --key_type 0x3::token::TokenDataId --value_type 0x3::token::TokenData --key_value {"creator":"0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6","collection":"velor_punks","name":"geek_token"}
