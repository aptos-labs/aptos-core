#[test_only]
module Std::GUIDTests {
    use Std::GUID;

    #[test(s = @0x42)]
    fun test_basics(s: signer) {
        let id1 = GUID::create(&s);
        let id2 = GUID::create(&s);

        assert(&id1 != &id2, 0);

        assert(GUID::creator_address(&id1) == @0x42, 1);
        assert(GUID::creator_address(&id2) == @0x42, 2);

        assert(GUID::creation_num(&id1) == 0, 3);
        assert(GUID::creation_num(&id2) == 1, 4);

        assert(GUID::get_next_creation_num(@0x42) == 2, 5);
        assert(GUID::get_next_creation_num(@0x0) == 0, 6);
    }

     #[test(s = @0x42)]
    fun test_id(s: signer) {
        let guid = GUID::create(&s);
        let id1 = GUID::id(&guid);

        assert(GUID::id_creator_address(&id1) == GUID::creator_address(&guid), 3);
        assert(GUID::id_creation_num(&id1) == GUID::creation_num(&guid), 4);
        assert(GUID::eq_id(&guid, &id1), 2);

        let id2 = GUID::create_id(@0x42, 0);
        assert(&id1 == &id2, 0);

        let _ids_are_copyable = copy id1;
    }
}
