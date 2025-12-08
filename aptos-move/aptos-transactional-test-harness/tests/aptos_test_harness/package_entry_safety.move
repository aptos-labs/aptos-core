//# init --addresses alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f

//# publish --private-key alice
module alice::entry_points_safety {

    // Error: public(package) entry function is unsafe without attribute
    public(package) entry fun unsafe_package_entry() {
    }

    // Ok: public(package) entry function allowed with attribute
    #[lint::allow_unsafe_package_entry]
    public(package) entry fun safe_package_entry() {
    }

    // Ok: private entry function
    entry fun private_entry() {
    }

    // Ok: public entry function
    public entry fun public_entry() {
    }

    // Ok: non-entry public(package) function
    public(package) fun package_non_entry() {
    }
}
