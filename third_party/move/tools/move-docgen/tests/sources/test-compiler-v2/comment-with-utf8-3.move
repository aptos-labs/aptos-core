address 0x2 {
module TestViz {
    /// 这是一个公用函数
    public fun this_is_a_public_fun() { }

    // /// 这是一个公用，朋友函数
    // public(friend) fun this_is_a_public_friend_fun() {}

    /// 这是一个公用，入口函数
    public entry fun this_is_a_public_script_fun() {}

    /// 这是一个私有函数
    fun this_is_a_private_fun() {}
}

module TestViz1 {
    /// # 算法注释
    /// ```
    /// 代码块
    /// ```
    /// 然后內联函数
    public fun main() { }
}
module TestViz2 {
    /** # 算法注释
     ```
     代码块
     ```
     然后內联函数
    */
    public fun main() { }
}

}
