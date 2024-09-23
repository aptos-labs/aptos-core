//# init --addresses Alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys Alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f

//# publish --private-key Alice
module Alice::M {
    public fun init_module(value: u64): u64 { value }

    #[view]
    fun view(_:&mut signer,value: u64): u64 { value }
}
