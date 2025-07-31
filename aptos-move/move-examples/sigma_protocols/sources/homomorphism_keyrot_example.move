module sigma_protocols::homomorphism_keyrot_example {
    use sigma_protocols::public_statement::PublicStatement;
    use sigma_protocols::representation_vec::{RepresentationVec, new_representation_vec};
    use sigma_protocols::homomorphism::{Self, SecretWitness, Proof};

    fun psiKeyRot(_stmt: &PublicStatement, _w: &SecretWitness): RepresentationVec {
        // TODO: impl
        new_representation_vec(vector[])
    }

    fun fKeyRot(_stmt: &PublicStatement): RepresentationVec {
        // TODO: impl
        new_representation_vec(vector[])
    }

    public fun keyrot_verify(stmt: &PublicStatement, proof: &Proof): bool {
        homomorphism::verify(
            b"my_example_application_s_domain_separator",
            b"keyrot",
            |_stmt, _wit| psiKeyRot(_stmt, _wit),
            |_stmt| fKeyRot(_stmt),
            stmt,
            proof
        )
    }
}
