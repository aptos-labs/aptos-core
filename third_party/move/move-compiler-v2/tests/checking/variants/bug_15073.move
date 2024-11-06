module 0x815::m {

    public fun init_registration(
        creator: &signer,
        tool_type: ToolType
    ) {
        match(tool_type) {
            ToolType::Http { uri } => {

            },
        }

    }
}
