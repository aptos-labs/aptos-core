spec 0x42::file_signature_alias {

    spec identity(value: 0x2::X::T): 0x2::X::T {
        use 0x2::X;
        pragma opaque = true;
        ensures [inferred] result == value;
        aborts_if [inferred] false;
    }


    spec keep_local(value: Local): Local {
        pragma opaque = true;
        ensures [inferred] result == value;
        aborts_if [inferred] false;
    }

}
/*
Verification:
warning: unused alias
  ┌─ file_signature_alias.spec.move:4:18
  │
4 │         use 0x2::X;
  │                  ^ Unused 'use' of alias 'X'. Consider removing it
*/
