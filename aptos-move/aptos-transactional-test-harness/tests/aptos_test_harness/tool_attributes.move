//# init --addresses Alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys Alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f

//# publish --private-key Alice
module Alice::allowed_attributes {
    #[mutation::skip]
    fun blah() {}

    #[fmt::skip]
    #[mutation::skip]
    fun blah_blah() {}
}
