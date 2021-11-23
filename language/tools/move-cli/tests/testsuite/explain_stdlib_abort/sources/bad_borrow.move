script {
    use Std::Vector;
    fun bad_borrow() {
        let v = Vector::empty<bool>();
        let _ref = Vector::borrow(&v, 0);
    }
}
