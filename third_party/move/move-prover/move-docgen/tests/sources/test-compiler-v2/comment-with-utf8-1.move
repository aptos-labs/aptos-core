script {
/**
    这是一个拥有utf8字符的文档注释
*/
/// 这个脚本只有一个函数
fun some<T>(_account: signer) {
    // 这个函数会abort
    abort 1
}
}
