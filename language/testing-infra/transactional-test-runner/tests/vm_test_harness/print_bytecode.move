//# print-bytecode
main() {
label b0:
    return;
}

//# print-bytecode --input=module
module 0x42.M {
    f() {
    label b0:
        return;
    }
}
