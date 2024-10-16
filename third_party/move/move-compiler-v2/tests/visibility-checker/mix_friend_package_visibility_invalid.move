address 0x42 {
    module A {
        public(package) fun foo() {

        }

        public(friend) fun bar() {

        }
    }
}

address 0x43 {
    module A {
        public(friend) fun foo() {

        }

        public(package) fun bar() {

        }
    }
}

address 0x44 {
    module A {
        friend 0x44::B;

        public(package) fun bar() {

        }
    }

    module B {}
}
