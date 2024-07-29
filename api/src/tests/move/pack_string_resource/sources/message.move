module addr::message {
    use 0x1::string;

    struct MessageHolder has key {
        message: string::String,
    }

    public entry fun set_message(account: signer, msg: vector<u8>) {
        let message = string::utf8(msg);
        move_to(&account, MessageHolder {
            message,
        });
    }
}
