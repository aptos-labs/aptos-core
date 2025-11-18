BEGIN {
    depth = 0
}

$1 == "BEGIN" {
    stack[depth] = $2
    depth++
}

$1 == "END" {
    # print current stack
    out = stack[0]
    for (i = 1; i < depth; i++) {
        out = out ";" stack[i]
    }
    print out, $2

    # pop
    depth--
}
