module 0x42::requires {
    public fun g() {
        f();
    }

    public fun f() {
    }
    spec f {
    }

    spec module {
        apply RequiresFalse to f;
    }

    spec schema RequiresFalse {
        requires false;
    }
}
