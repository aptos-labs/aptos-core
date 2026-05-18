module 0x42::enum_types {

    use std::string;

    enum MessageHolder has key, drop {
        Empty,
        Message{
            message: string::String,
        }
        NewMessage{
            message: string::String,
        }
    }
}
