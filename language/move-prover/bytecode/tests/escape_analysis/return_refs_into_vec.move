// dep: ../../move-stdlib/sources/Vector.move

module 0x1::ReturnRefsIntoVec {
    use Std::Vector;

    // should not complain
    fun return_vec_index_immut(v: &vector<u64>): &u64 {
        Vector::borrow(v, 0)
    }

    // should complain
    fun return_vec_index_mut(v: &mut vector<u64>): &mut u64 {
        Vector::borrow_mut(v, 0)
    }

}
