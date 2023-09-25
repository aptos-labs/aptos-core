module 0x42::M {

  struct S {
    x: u64,
    y: bool,
  }

  spec module {

    fun undeclared_name() : num {
      x // Undeclared simple name.
    }

    fun undeclared_fun(): num {
      not_declared() // Undeclared function.
    }

    fun wrong_result_type(): num {
      false // Wrong result type.
    }

    fun no_overload(x: vector<num>, y: vector<num>): bool {
      x > y // No matching function.
    }

    fun wrong_result_type2(): (num, bool) {
      false // Wrong result type tuple.
    }

    fun wrongly_typed_callee(x: num, y: bool): num { x }
    fun wrongly_typed_caller(): num {
      wrongly_typed_callee(1, 1) // Wrongly typed function application
    }

    fun wrongly_typed_fun_arg_callee(f: |num|num): num { 0 }
    fun wrongly_typed_fun_arg_caller(): num {
      wrongly_typed_fun_arg_callee(|x| false) // Wrongly typed function argument.
    }

    fun wrong_instantiation<T1, T2>(x: T1): T1 { x }
    fun wrong_instantiation_caller(x: u64): u64 {
      wrong_instantiation<u64>(x) // Wrong instantiation
    }
  }
}
