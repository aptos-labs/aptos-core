address 0x42 {
module mod1 {
    struct C { }
    const C: u64 = 0;
    public fun mod1() {}
}
}

address 0x41 {
module N {
    use 0x42::mod1;
    use 0x42::mod1::C as D;
    use 0x42::mod1::C as C;
    use 0x42::mod1::mod1;

    fun f1(): 0x42::mod1::C {
	mod1();
	C;
	{
	    use 0x42::mod1::C;
	    C
	};
	D
    }
}
}


script {
    use 0x42::mod1;
    use 0x42::mod1::C as mod1;
    use 0x42::mod1::C as C;
    use 0x42::mod1::mod1;

    fun f1(): 0x42::mod1::C {
	mod1();
	C;
	{
	    use 0x42::mod1::C;
	    C
	}
    }
}
