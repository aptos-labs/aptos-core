module 0xc0ffee::complexity {
    // complexity: 12
    public fun high_complexity(a: bool, b: bool): bool {
        assert!(a || b, 100); // +1
        if (a) { // +1
            return b; // +1
        };

        if (b) { // +1
            return a; // +1
        };

        let c = a && b;

        loop { // +1
            if (c) { // +1
                return c; // +1
            } else {
                if (a || b) { // +1
                    return a; // +1
                };
            };
            break; // +1
        };
        a
    }


    // complexity: 1
    public fun low_complexity(): bool {
        let b = 1;
        inline_fun(b) == 1
    }

    // complexity: 11
    inline fun inline_fun(b: u64): u64 {
            if (b == 1) { // +1
                1
            }else if (b == 2) { // +1
                0
            }else if (b == 3) { // +1
                1
            }else if (b == 4) { // +1
                0
            }else if (b == 5) { // +1
                1
            }else if (b == 6) { // +1
                0
            }else if (b == 7) { // +1
                1
            }else if (b == 8) { // +1
                0
            }else if (b == 9) { // +1
                1
            }else if (b == 10) { // +1
                0
            }else {
                12
            }
    }

    // complexity: 1
    public fun simple_function(): u64 {
        42
    }

    // complexity: 2
    public fun single_if(x: u64): u64 {
        if (x > 0) { // +1
            x * 2
        } else {
            0
        }
    }

    // complexity: 4
    public fun multiple_if_else(x: u64): u64 {
        if (x == 1) { // +1
            1
        } else if (x == 2) { // +1
            4
        } else if (x == 3) { // +1
            9
        } else {
            0
        }
    }

    // complexity: 4
    public fun simple_while(x: u64): u64 {
        let y = x;
        if (y == 0) { // +1
            return 1; // +1
        };
        while (y > 1) { // +1
            y = y - 1;
        };
        y
    }

    // complexity: 4
    public fun while_with_conditions(x: u64): u64 {
        let y = x;
        while (y > 0) { // +1
            if (y % 2 == 0) { // +1
                y = y / 2;
            } else if (y > 10) { // +1
                y = y - 5;
            } else {
                y = y - 1;
            }
        };
        y
    }

    // complexity: 4
    public fun while_with_nested_conditions(x: u64): u64 {
        let y = x;
        while (y != 0) { // +1
            if (y % 3 == 0) { // +1
                y = y / 3;
            } else if (y % 2 == 0) { // +1
                y = y / 2;
            } else {
                y = y - 1;
            }
        };
        y
    }

    // complexity: 5
    public fun while_with_nested_ifs(n: u64): u64 {
        let sum = 0;
        let i = 0;
        while (i < n) { // +1
            if (i % 2 == 0) { // +1
                if (i % 4 == 0) { // +1
                    sum = sum + i * 2;
                } else {
                    sum = sum + i;
                }
            } else {
                if (i % 3 == 0) { // +1
                    sum = sum + i / 2;
                } else {
                    sum = sum + 1;
                }
            };
            i = i + 1;
        };
        sum
    }

    // complexity: 8
    public fun complex_nested_structure(x: u64, y: u64): u64 {
        let a = x;
        if (a > 100) { // +1
            return a + y; // +1
        } else if (a > 50) { // +1
            while (a > y) { // +1
                if (a % 2 == 0) { // +1
                    if (y > 10) { // +1
                        a = a - y;
                    } else {
                        a = a - 1;
                    }
                } else {
                    if (y < 5) { // +1
                        a = a / 2;
                    } else {
                        a = a - 3;
                    }
                }
            }
        };
        a
    }

    // complexity: 7
    public fun nested_while_loops(n: u64): u64 {
        let result = 0;
        let i = 1;
        while (i < n) { // +1
            let temp = i;
            while (temp > 0) { // +1
                if (temp % 2 == 0) { // +1
                    if (temp > 10) { // +1
                        temp = temp / 2;
                    } else {
                        temp = temp - 1;
                    }
                } else {
                    if (temp > 5) { // +1
                        temp = temp - 2;
                    } else {
                        break; // +1
                    }
                }
            };
            result = result + temp;
            i = i + 1;
        };
        result
    }

    // complexity: 7
    public fun very_complex_while(): u64 {
        let x = 100;
        let y = 50;

        while (x != 0) { // +1
            if (x > y) { // +1
                if (x % 2 == 0) { // +1
                    if (y > 10) { // +1
                        x = x - y;
                    } else {
                        x = x / 2;
                    }
                } else {
                    if (y < 20) { // +1
                        x = x - 3;
                    } else {
                        y = y - 1;
                    }
                }
            } else if (y > x) { // +1
                y = y - x;
            } else {
                x = x - 1;
            }
        };
        x + y
    }

    // complexity: 6
    public fun while_with_early_returns(x: u64): u64 {
        let y = x;
        while (y > 0) { // +1
            if (y > 100) { // +1
                return y; // +1
            };
            if (y % 10 == 0) { // +1
                y = y / 10;
            } else if (y % 5 == 0) { // +1
                y = y / 5;
            } else {
                y = y - 1;
            }
        };
        0
    }

    // complexity: 10
    public fun switch_like_structure(n: u64): u64 {
        let result = 0;
        let i = 0;
        while (i < n) { // +1
            if (i == 0) { // +1
                result = result + 1;
            } else if (i == 1) { // +1
                result = result + 2;
            } else if (i == 2) { // +1
                result = result + 3;
            } else if (i == 3) { // +1
                result = result + 5;
            } else if (i == 4) { // +1
                result = result + 8;
            } else if (i == 5) { // +1
                result = result + 13;
            } else if (i == 6) { // +1
                result = result + 21;
            } else if (i == 7) { // +1
                result = result + 34;
            } else {
                result = result + i;
            };
            i = i + 1;
        };
        result
    }

    // complexity: 5
    public fun double_nested_while_loops(n: u64, m: u64): u64 {
        let sum = 0;
        let i = 0;
        while (i < n) { // +1
            let j = 0;
            while (j < m) { // +1
                if (i == j) { // +1
                    sum = sum + i * j;
                } else if (i > j) { // +1
                    sum = sum + i - j;
                } else {
                    sum = sum + j - i;
                };
                j = j + 1;
            };
            i = i + 1;
        };
        sum
    }

    // complexity: 3
    public fun while_with_nested_while(x: u64): u64 {
        let y = x;
        while (y != 0) { // +1
            while (y > 10) { // +1
                y = y - 5;
            };
            y = y - 1;
        };
        y
    }

    // complexity: 10
    public fun extremely_complex_function(a: u64, b: u64, c: bool): u64 {
        let x = a;
        let y = b;
        while (x > 0 && y > 0) { // +1
            if (c) { // +1
                if (x > y) { // +1
                    if (x % 2 == 0) { // +1
                        if (y % 3 == 0) { // +1
                            x = x - y;
                        } else {
                            x = x / 2;
                        }
                    } else {
                        if (y > 10) { // +1
                            y = y - 5;
                        } else {
                            x = x - 1;
                        }
                    }
                } else {
                    if (y % 2 == 0) { // +1
                        y = y / 2;
                    } else {
                        y = y - 1;
                    }
                }
            } else {
                if (x < 10) { // +1
                    x = 0;
                } else if (y < 10) { // +1
                    y = 0;
                } else {
                    x = x - 1;
                    y = y - 1;
                }
            }
        };
        x + y
    }

    // complexity: 3
    public fun simple_while_loop(n: u64): u64 {
        let sum = 0;
        let i = 0;
        while (i < n) { // +1
            if (i % 2 == 0) { // +1
                sum = sum + i;
            };
            i = i + 1;
        };
        sum
    }

    // complexity: 7
    public fun nested_while_with_many_conditions(n: u64, m: u64): u64 {
        let result = 0;
        let i = 0;
        while (i < n) { // +1
            let j = 0;
            while (j < m) { // +1
                if (i + j == 0) { // +1
                    result = result + 1;
                } else if (i + j < 5) { // +1
                    result = result + 2;
                } else if (i + j < 10) { // +1
                    result = result + 3;
                } else if (i + j < 20) { // +1
                    result = result + 4;
                } else {
                    result = result + 5;
                };
                j = j + 1;
            };
            i = i + 1;
        };
        result
    }

    // complexity: 6
    public fun while_while_combination(x: u64): u64 {
        let total = 0;
        let y = x;
        while (y > 0) { // +1
            let i = 0;
            while (i < y) { // +1
                if (i == 0) { // +1
                    total = total + 1;
                } else if (i % 2 == 0) { // +1
                    total = total + i;
                } else if (i % 3 == 0) { // +1
                    total = total + i * 2;
                } else {
                    total = total + i / 2;
                };
                i = i + 1;
            };
            y = y - 1;
        };
        total
    }

    // complexity: 10
    public fun complex_state_machine(): u64 {
        let state = 0;
        let counter = 0;

        loop { // +1
            if (state == 0) { // +1
                state = 1;
                counter = counter + 1;
            } else if (state == 1) { // +1
                state = 2;
                counter = counter + 2;
            } else if (state == 2) { // +1
                state = 3;
                counter = counter + 3;
            } else if (state == 3) { // +1
                state = 4;
                counter = counter + 4;
            } else if (state == 4) { // +1
                state = 5;
                counter = counter + 5;
            } else if (state == 5) { // +1
                state = 6;
                counter = counter + 6;
            } else if (state == 6) { // +1
                state = 7;
                counter = counter + 7;
            } else if (state == 7) { // +1
                state = 8;
                counter = counter + 8;
            } else {
                state = 0;
            }
        };
        counter
    }

    // complexity: 14
    public fun triple_nested_while_loops(a: u64, b: u64, c: u64): u64 {
        let sum = 0;
        let i = 0;
        while (i < a) { // +1
            let j = 0;
            while (j < b) { // +1
                let k = 0;
                while (k < c) { // +1
                    if (i == 0 && j == 0 && k == 0) { // +1
                        sum = sum + 100;
                    } else if (i == j && j == k) { // +1
                        sum = sum + 50;
                    } else if (i > j && j > k) { // +1
                        sum = sum + 25;
                    } else if (i < j && j < k) { // +1
                        sum = sum + 10;
                    } else if (i % 2 == 0) { // +1
                        sum = sum + i;
                    } else if (j % 2 == 0) { // +1
                        sum = sum + j;
                    } else if (k % 2 == 0) { // +1
                        sum = sum + k;
                    } else if (i + j > k) { // +1
                        sum = sum + 5;
                    } else if (i + k > j) { // +1
                        sum = sum + 3;
                    } else if (j + k > i) { // +1
                        sum = sum + 2;
                    } else {
                        sum = sum + 1;
                    };
                    k = k + 1;
                };
                j = j + 1;
            };
            i = i + 1;
        };
        sum
    }

    // complexity: 8
    public fun nested_while_combination(x: u64, y: u64): u64 {
        let a = x;
        let b = y;
        while (a > 0) { // +1
            while (b != 0) { // +1
                if (a > b) { // +1
                    a = a - b;
                } else if (b > a) { // +1
                    b = b - a;
                } else if (a == b) { // +1
                    return a; // +1
                } else if (a % 2 == 0) { // +1
                    a = a / 2;
                } else {
                    b = b + 1;
                }
            };
            a = a - 1;
        };
        a + b
    }

    // complexity: 10
    public fun while_with_early_returns_v2(n: u64): u64 {
        let i = 0;
        while (i < n) { // +1
            if (i == 42) { // +1
                return i * 2; // +1
            } else if (i > 100) { // +1
                return i / 2; // +1
            } else if (i % 10 == 0) { // +1
                return i + 10; // +1
            } else if (i % 7 == 0) { // +1
                return i - 5; // +1
            };
            i = i + 1;
        };
        0
    }

    // complexity: 10
    public fun deeply_nested_conditions(x: u64, y: u64, z: bool): u64 {
        let a = x;
        let b = y;
        while (a > 0 || b > 0) { // +1
            if (z) { // +1
                if (a > 50) { // +1
                    if (b > 30) { // +1
                        if (a % 2 == 0) { // +1
                            a = a / 2;
                        } else {
                            a = a - 1;
                        }
                    } else {
                        if (b % 3 == 0) { // +1
                            b = b * 2;
                        } else {
                            b = b + 1;
                        }
                    }
                } else {
                    if (a < 10) { // +1
                        a = 0;
                    } else {
                        a = a - 5;
                    }
                }
            } else {
                if (b > a) { // +1
                    if (b % 2 == 0) { // +1
                        b = b / 2;
                    } else {
                        b = b - 3;
                    }
                } else {
                    a = a - 1;
                }
            }
        };
        a + b
    }

    // complexity: 4
    public fun while_with_breaks_and_continues(n: u64): u64 {
        let i = 0;
        let sum = 0;

        while (i < n) { // +1
            if (i % 2 == 0) { // +1
                i = i + 1;

            } else {
                if (i % 3 == 0) { // +1
                    sum = sum + i * 2;
                } else {
                    sum = sum + i;
                };
                i = i + 1;
            }
        };
        sum
    }

    // complexity: 8
    public fun double_while_loops(a: u64, b: u64): u64 {
        let x = a;
        let y = b;
        while (x > 0) { // +1
            while (y > 0) { // +1
                if (x == y) { // +1
                    return x + y; // +1
                } else if (x > y) { // +1
                    x = x - y;
                } else if (y > x) { // +1
                    y = y - x;
                } else if (x % 2 == 0) { // +1
                    x = x / 2;
                } else {
                    y = y - 1;
                }
            };
            x = x - 1;
        };
        x + y
    }

    // complexity: 13
    public fun massive_switch_like(n: u64): u64 {
        let result = 0;
        let i = 0;
        while (i < n) { // +1
            if (i % 12 == 0) { // +1
                result = result + 12;
            } else if (i % 11 == 0) { // +1
                result = result + 11;
            } else if (i % 10 == 0) { // +1
                result = result + 10;
            } else if (i % 9 == 0) { // +1
                result = result + 9;
            } else if (i % 8 == 0) { // +1
                result = result + 8;
            } else if (i % 7 == 0) { // +1
                result = result + 7;
            } else if (i % 6 == 0) { // +1
                result = result + 6;
            } else if (i % 5 == 0) { // +1
                result = result + 5;
            } else if (i % 4 == 0) { // +1
                result = result + 4;
            } else if (i % 3 == 0) { // +1
                result = result + 3;
            } else if (i % 2 == 0) { // +1
                result = result + 2;
            } else {
                result = result + 1;
            };
            i = i + 1;
        };
        result
    }

    // complexity: 10
    public fun while_while_nested_complex(n: u64): u64 {
        let total = 0;
        let i = 0;
        while (i < n) { // +1
            let temp = i;
            while (temp > 0) { // +1
                if (temp > 100) { // +1
                    temp = temp / 10;
                } else if (temp > 50) { // +1
                    temp = temp / 5;
                } else if (temp > 20) { // +1
                    temp = temp / 2;
                } else if (temp > 10) { // +1
                    temp = temp - 5;
                } else if (temp > 5) { // +1
                    temp = temp - 2;
                } else if (temp > 1) { // +1
                    temp = temp - 1;
                } else {
                    break; // +1
                }
            };
            total = total + temp;
            i = i + 1;
        };
        total
    }

    // complexity: 16
    public fun ultra_complex_nested(x: u64, y: u64, z: u64): u64 {
        let a = x;
        let b = y;
        let c = z;
        while (!(a == 0 && b == 0 && c == 0)) { // +1
            if (a > b && b > c) { // +1
                if (a % 2 == 0) { // +1
                    if (b % 2 == 0) { // +1
                        a = a / 2;
                        b = b / 2;
                    } else {
                        a = a - b;
                    }
                } else {
                    if (c % 2 == 0) { // +1
                        c = c / 2;
                    } else {
                        a = a - 1;
                    }
                }
            } else if (b > a && a > c) { // +1
                if (b % 3 == 0) { // +1
                    b = b / 3;
                } else {
                    b = b - a;
                }
            } else if (c > a && a > b) { // +1
                if (c % 4 == 0) { // +1
                    c = c / 4;
                } else {
                    c = c - b;
                }
            } else if (a == b) { // +1
                if (a > c) { // +1
                    a = a - c;
                } else {
                    c = c - a;
                }
            } else if (b == c) { // +1
                if (b > a) { // +1
                    b = b - a;
                } else {
                    a = a - b;
                }
            } else if (a == c) { // +1
                if (a > b) { // +1
                    a = a - b;
                } else {
                    b = b - a;
                }
            } else {
                a = a - 1;
                b = b - 1;
                c = c - 1;
            }
        };
        a + b + c
    }

    // complexity: 4
    public fun simple_triple_while(a: u64, b: u64, c: u64): u64 {
        let count = 0;
        let i = 0;
        while (i < a) { // +1
            let j = 0;
            while (j < b) { // +1
                let k = 0;
                while (k < c) { // +1
                    count = count + 1;
                    k = k + 1;
                };
                j = j + 1;
            };
            i = i + 1;
        };
        count
    }

    // complexity: 6
    public fun mixed_while_with_conditions(n: u64): u64 {
        let result = 0;
        let x = n;
        while (x > 10) { // +1
            let i = 0;
            while (i < 5) { // +1
                if (i == 0) { // +1
                    result = result + x;
                } else if (i == 1) { // +1
                    result = result + x / 2;
                } else if (i == 2) { // +1
                    result = result + x / 3;
                } else {
                    result = result + 1;
                };
                i = i + 1;
            };
            x = x - 5;
        };
        result
    }

    // complexity: 17
    public fun enormous_switch(n: u64): u64 {
        let sum = 0;
        let i = 0;
        while (i < n) { // +1
            if (i % 16 == 0) { // +1
                sum = sum + 16;
            } else if (i % 15 == 0) { // +1
                sum = sum + 15;
            } else if (i % 14 == 0) { // +1
                sum = sum + 14;
            } else if (i % 13 == 0) { // +1
                sum = sum + 13;
            } else if (i % 12 == 0) { // +1
                sum = sum + 12;
            } else if (i % 11 == 0) { // +1
                sum = sum + 11;
            } else if (i % 10 == 0) { // +1
                sum = sum + 10;
            } else if (i % 9 == 0) { // +1
                sum = sum + 9;
            } else if (i % 8 == 0) { // +1
                sum = sum + 8;
            } else if (i % 7 == 0) { // +1
                sum = sum + 7;
            } else if (i % 6 == 0) { // +1
                sum = sum + 6;
            } else if (i % 5 == 0) { // +1
                sum = sum + 5;
            } else if (i % 4 == 0) { // +1
                sum = sum + 4;
            } else if (i % 3 == 0) { // +1
                sum = sum + 3;
            } else if (i % 2 == 0) { // +1
                sum = sum + 2;
            } else {
                sum = sum + 1;
            };
            i = i + 1;
        };
        sum
    }

    // complexity: 16
    public fun maximum_complexity_test(a: u64, b: u64, c: u64, flag: bool): u64 {
        let x = a;
        let y = b;
        let z = c;
        while (x > 0) { // +1
            while (y > 0) { // +1
                if (flag) { // +1
                    if (x > 100) { // +1
                        if (y > 50) { // +1
                            if (z > 25) { // +1
                                x = x - z;
                            } else {
                                x = x / 2;
                            }
                        } else {
                            if (z % 2 == 0) { // +1
                                y = y + z;
                            } else {
                                y = y - 1;
                            }
                        }
                    } else {
                        if (x > 50) { // +1
                            if (y % 3 == 0) { // +1
                                x = x * 2;
                            } else {
                                x = x + y;
                            }
                        } else {
                            if (z > 10) { // +1
                                z = z / 2;
                            } else {
                                z = z + 1;
                            }
                        }
                    }
                } else {
                    if (y > x) { // +1
                        if (z > y) { // +1
                            y = y - z;
                        } else {
                            y = y / 2;
                        }
                    } else {
                        if (x % 2 == 0) { // +1
                            if (y % 2 == 0) { // +1
                                x = x / 2;
                                y = y / 2;
                            } else {
                                x = x - 1;
                            }
                        } else {
                            if (z % 3 == 0) { // +1
                                z = z / 3;
                            } else {
                                z = z + x;
                            }
                        }
                    }
                }
            };
            x = x - 1;
        };
        x + y + z
    }



    // complexity: 2
    public fun simple_for_loop(n: u64): u64 {
        let sum = 0;
        for (i in 0..n) { // +1
            sum = sum + i;
        };
        sum
    }

    // complexity: 3
    public fun for_with_simple_if(n: u64): u64 {
        let sum = 0;
        for (i in 0..n) { // +1
            if (i % 2 == 0) { // +1
                sum = sum + i;
            }
        };
        sum
    }

    // complexity: 4
    public fun for_with_multiple_conditions(n: u64): u64 {
        let result = 0;
        for (i in 0..n) { // +1
            if (i % 3 == 0) { // +1
                result = result + 3;
            } else if (i % 2 == 0) { // +1
                result = result + 2;
            } else {
                result = result + 1;
            }
        };
        result
    }

    // complexity: 4
    public fun nested_for_loops(n: u64, m: u64): u64 {
        let count = 0;
        for (i in 0..n) { // +1
            for (j in 0..m) { // +1
                if (i == j) { // +1
                    count = count + 1;
                }
            }
        };
        count
    }

    // complexity: 6
    public fun nested_for_with_conditions(n: u64, m: u64): u64 {
        let sum = 0;
        for (i in 0..n) { // +1
            for (j in 0..m) { // +1
                if (i + j == 0) { // +1
                    sum = sum + 10;
                } else if (i > j) { // +1
                    sum = sum + i;
                } else if (j > i) { // +1
                    sum = sum + j;
                } else {
                    sum = sum + 1;
                }
            }
        };
        sum
    }

    // complexity: 6
    public fun triple_for_loops(a: u64, b: u64, c: u64): u64 {
        let total = 0;
        for (i in 0..a) { // +1
            for (j in 0..b) { // +1
                for (k in 0..c) { // +1
                    if (i + j + k == 0) { // +1
                        total = total + 100;
                    } else if (i * j * k > 0) { // +1
                        total = total + i * j * k;
                    }
                }
            }
        };
        total
    }

    // complexity: 4
    public fun for_with_continue(n: u64): u64 {
        let sum = 0;
        for (i in 0..n) { // +1
            if (i % 2 == 0 && i > 10) { // +1
                continue; // +1
            };
            sum = sum + i;
        };
        sum
    }

    // complexity: 4
    public fun for_with_break(n: u64): u64 {
        let sum = 0;
        for (i in 0..n) { // +1
            if (i > 50 && sum > 1000) { // +1
                break; // +1
            };
            sum = sum + i;
        };
        sum
    }

    // complexity: 6
    public fun for_with_break_and_continue(n: u64): u64 {
        let sum = 0;
        for (i in 0..n) { // +1
            if (i % 3 == 0 && i > 0) { // +1
                continue; // +1
            };
            if (sum > 500) { // +1
                break; // +1
            };
            sum = sum + i;
        };
        sum
    }

    // complexity: 4
    public fun for_with_while(n: u64): u64 {
        let result = 0;
        for (i in 0..n) { // +1
            let temp = i;
            while (temp > 0) { // +1
                if (temp % 2 == 0) { // +1
                    result = result + temp;
                };
                temp = temp - 1;
            };
        };
        result
    }

    // complexity: 7
    public fun for_while_complex(n: u64): u64 {
        let total = 0;
        for (i in 0..n) { // +1
            let x = i;
            while (x > 0) { // +1
                if (x % 5 == 0) { // +1
                    total = total + 5;
                } else if (x % 4 == 0) { // +1
                    total = total + 4;
                } else if (x % 3 == 0) { // +1
                    total = total + 3;
                } else if (x % 2 == 0) { // +1
                    total = total + 2;
                } else {
                    total = total + 1;
                };
                x = x - 1;
            };
        };
        total
    }

    // complexity: 11
    public fun for_with_early_return(n: u64): u64 {
        let sum = 0;
        for (i in 0..n) { // +1
            if (i == 42) { // +1
                return sum + 1000; // +1
            } else if (i % 10 == 0) { // +1
                sum = sum + 100;
            } else if (i % 9 == 0) { // +1
                sum = sum + 90;
            } else if (i % 8 == 0) { // +1
                sum = sum + 80;
            } else if (i % 7 == 0) { // +1
                sum = sum + 70;
            } else if (i % 6 == 0) { // +1
                sum = sum + 60;
            } else if (i % 5 == 0) { // +1
                sum = sum + 50;
            } else if (i % 4 == 0) { // +1
                sum = sum + 40;
            } else {
                sum = sum + i;
            }
        };
        sum
    }

    // complexity: 13
    public fun complex_nested_for_with_boolean_ops(n: u64, m: u64): u64 {
        let result = 0;
        for (i in 0..n) { // +1
            for (j in 0..m) { // +1
                if (i == 0 && j == 0) { // +1
                    result = result + 1000;
                } else if (i % 7 == 0) { // +1
                    result = result + 70;
                } else if (j % 6 == 0) { // +1
                    result = result + 60;
                } else if (i % 5 == 0) { // +1
                    result = result + 50;
                } else if (j % 4 == 0) { // +1
                    result = result + 40;
                } else if (i % 3 == 0) { // +1
                    result = result + 30;
                } else if (j % 2 == 0) { // +1
                    result = result + 20;
                } else if (i > j) { // +1
                    result = result + 10;
                } else if (j > i) { // +1
                    result = result + 5;
                } else if (i == j) { // +1
                    result = result + 2;
                } else {
                    result = result + 1;
                }
            }
        };
        result
    }

    // complexity: 15
    public fun massive_triple_for_with_conditions(a: u64, b: u64, c: u64): u64 {
        let sum = 0;
        for (i in 0..a) { // +1
            for (j in 0..b) { // +1
                for (k in 0..c) { // +1
                    if (i == 0 && j == 0 && k == 0) { // +1
                        sum = sum + 10000;
                    } else if (i % 11 == 0) { // +1
                        sum = sum + 110;
                    } else if (j % 10 == 0) { // +1
                        sum = sum + 100;
                    } else if (k % 9 == 0) { // +1
                        sum = sum + 90;
                    } else if (i % 8 == 0) { // +1
                        sum = sum + 80;
                    } else if (j % 7 == 0) { // +1
                        sum = sum + 70;
                    } else if (k % 6 == 0) { // +1
                        sum = sum + 60;
                    } else if (i % 5 == 0) { // +1
                        sum = sum + 50;
                    } else if (j % 4 == 0) { // +1
                        sum = sum + 40;
                    } else if (k % 3 == 0) { // +1
                        sum = sum + 30;
                    } else if (i % 2 == 0) { // +1
                        sum = sum + 20;
                    } else {
                        sum = sum + 1;
                    }
                }
            }
        };
        sum
    }

    // complexity: 9
    public fun for_with_inner_loop(n: u64): u64 {
        let total = 0;
        for (i in 0..n) { // +1
            let counter = i;
            loop { // +1
                if (counter == 0) { // +1
                    break; // +1
                } else if (counter % 5 == 0) { // +1
                    total = total + 50;
                } else if (counter % 4 == 0) { // +1
                    total = total + 40;
                } else if (counter % 3 == 0) { // +1
                    total = total + 30;
                } else if (counter % 2 == 0) { // +1
                    total = total + 20;
                } else {
                    total = total + 10;
                };
                counter = counter - 1;
            };
        };
        total
    }

    // complexity: 10
    public fun for_while_switch_like(n: u64): u64 {
        let result = 0;
        for (i in 0..n) { // +1
            let temp = i % 10;
            while (temp > 0) { // +1
                if (temp == 9) { // +1
                    result = result + 90;
                } else if (temp == 8) { // +1
                    result = result + 80;
                } else if (temp == 7) { // +1
                    result = result + 70;
                } else if (temp == 6) { // +1
                    result = result + 60;
                } else if (temp == 5) { // +1
                    result = result + 50;
                } else if (temp == 4) { // +1
                    result = result + 40;
                } else if (temp == 3) { // +1
                    result = result + 30;
                } else {
                    result = result + temp;
                };
                temp = temp - 1;
            };
        };
        result
    }

    // complexity: 62
    public fun comprehensive_control_flow_test(x: u64, y: u64, z: u64, flag: bool): u64 {
        assert!(x > 0, 1001); // +1
        assert!(y > 0, 1002); // +1
        assert!(z > 0, 1003); // +1
        assert!(x <= 1000, 1004); // +1

        let result = 0;
        let counter = 0;

        for (i in 0..x) { // +1
            assert!(i < 1000, 1005); // +1
            assert!(result < 1000000, 1006); // +1
            let temp = i;
            while (temp > 0) { // +1
                assert!(temp <= i, 1007); // +1
                if (temp % 5 == 0) { // +1
                    result = result + temp;
                } else if (temp % 3 == 0) { // +1
                    result = result + temp * 2;
                } else {
                    result = result + 1;
                };
                temp = temp - 1;
            };

            for (inner_i in 0..3) { // +1
                assert!(inner_i < 5, 1008); // +1
                if (inner_i % 2 == 0 && flag) { // +1
                    result = result + inner_i * 10;
                } else if (inner_i % 2 == 1 && !flag) { // +1
                    result = result + inner_i * 5;
                } else {
                    result = result + 2;
                };
            };
        };

        assert!(counter == 0, 1009); // +1
        loop { // +1
            assert!(counter < 100, 1010); // +1
            assert!(counter != 0, 1011); // +1
            if (counter >= y) { // +1
                break; // +1
            };

            for (j in 0..5) { // +1
                assert!(j < 10, 1012); // +1
                let inner_temp = j;
                while (inner_temp > 0) { // +1
                    assert!(inner_temp <= j, 1013); // +1
                    if (flag && inner_temp % 2 == 0) { // +1
                        result = result + inner_temp * counter;
                    } else if (!flag && inner_temp % 2 == 1) { // +1
                        result = result + inner_temp + counter;
                    } else {
                        result = result + 10;
                    };
                    inner_temp = inner_temp - 1;
                };

                let extra_counter = j;
                while (extra_counter < 10) { // +1
                    assert!(extra_counter >= j, 1014); // +1
                    if (extra_counter % 4 == 0) { // +1
                        result = result + 40;
                    } else if (extra_counter % 3 == 0) { // +1
                        result = result + 30;
                    } else {
                        result = result + extra_counter;
                    };
                    extra_counter = extra_counter + 1;
                };
            };
            counter = counter + 1;
        };

        assert!(counter > 0, 1015); // +1
        for (outer in 0..4) { // +1
            assert!(outer < 10, 1016); // +1
            assert!(outer != 0, 1017); // +1
            let first_temp = outer;
            while (first_temp > 0) { // +1
                assert!(first_temp <= outer, 1018); // +1
                if (first_temp % 2 == 0) { // +1
                    let second_temp = first_temp;
                    while (second_temp > 0) { // +1
                        assert!(second_temp <= first_temp, 1019); // +1
                        if (second_temp % 3 == 0 && flag) { // +1
                            result = result + second_temp * 3;
                        } else if (second_temp % 3 == 1 && !flag) { // +1
                            result = result + second_temp * 2;
                        } else {
                            result = result + second_temp;
                        };
                        second_temp = second_temp - 1;
                    };
                } else {
                    result = result + first_temp;
                };
                first_temp = first_temp - 1;
            };
        };

        let final_counter = 0;
        while (final_counter < z) { // +1
            assert!(final_counter < 50, 1020); // +1
            assert!(final_counter != 0, 1021); // +1
            for (k in 0..3) { // +1
                assert!(k < 5, 1022); // +1
                if (k == 0 && flag) { // +1
                    result = result + 100;
                } else if (k == 1 && !flag) { // +1
                    result = result + 200;
                } else {
                    result = result + k;
                };

                let nested_while_counter = k;
                while (nested_while_counter < 5) { // +1
                    assert!(nested_while_counter >= k, 1023); // +1
                    if (nested_while_counter % 2 == 0) { // +1
                        result = result + nested_while_counter * 2;
                    } else {
                        result = result + nested_while_counter;
                    };
                    nested_while_counter = nested_while_counter + 1;
                };
            };
            final_counter = final_counter + 1;
        };

        let last_counter = 0;
        while (last_counter < 3) { // +1
            assert!(last_counter != 0, 1024); // +1
            assert!(last_counter < 10, 1025); // +1
            for (final_i in 0..2) { // +1
                assert!(final_i < 3, 1026); // +1
                if (final_i == 0 && last_counter % 2 == 0) { // +1
                    result = result + 500;
                } else if (final_i == 1 && last_counter % 2 == 1) { // +1
                    result = result + 300;
                } else {
                    result = result + final_i + last_counter;
                };
            };
            last_counter = last_counter + 1;
        };

        assert!(result != 0, 1027); // +1
        assert!(result < 10000000, 1028); // +1
        result
    }


    #[lint::skip(cyclomatic_complexity)]
    public fun skipped_function(b: u64): u64 {
            if (b == 1) { // +1
                1
            }else if (b == 2) { // +1
                0
            }else if (b == 3) { // +1
                1
            }else if (b == 4) { // +1
                0
            }else if (b == 5) { // +1
                1
            }else if (b == 6) { // +1
                0
            }else if (b == 7) { // +1
                1
            }else if (b == 8) { // +1
                0
            }else if (b == 9) { // +1
                1
            }else if (b == 10) { // +1
                0
            }else {
                12
            }
    }

    // complexity: 10
    public fun block_return_1(b: u64): u64 {
            let a = if (b == 1) { // +1
                1
            }else if (b == 2) { // +1
                0
            }else if (b == 3) { // +1
                1
            }else if (b == 4) { // +1
                0
            }else if (b == 5) { // +1
                1
            }else if (b == 6) { // +1
                0
            }else if (b == 7) { // +1
                1
            }else if (b == 8) { // +1
                0
            }else if (b == 9) { // +1
                1
            }else {
                12
            };
            a
    }

    // complexity: 10
        public fun block_return_2(b: u64): u64 {
            let a = if (b == 1) { // +1
                1
            }else if (b == 2) { // +1
                0
            }else if (b == 3) { // +1
                1
            }else if (b == 4) { // +1
                0
            }else if (b == 5) { // +1
                1
            }else if (b == 6) { // +1
                0
            }else if (b == 7) { // +1
                1
            }else if (b == 8) { // +1
                0
            }else if (b == 9) { // +1
                1
            }else {
                12
            };
            return a
    }

    // complexity: 10
    public fun block_return_3(b: u64): u64 {
            let a = if (b == 1) { // +1
                1
            }else if (b == 2) { // +1
                0
            }else if (b == 3) { // +1
                1
            }else if (b == 4) { // +1
                0
            }else if (b == 5) { // +1
                1
            }else if (b == 6) { // +1
                0
            }else if (b == 7) { // +1
                1
            }else if (b == 8) { // +1
                0
            }else if (b == 9) { // +1
                1
            }else {
                12
            };
            {return a}
    }

        //complexity: 11
        public fun block_return_4(b: u64): u64 {
            let a = if (b == 1) { // +1
                1
            }else if (b == 2) { // +1
                0
            }else if (b == 3) { // +1
                1
            }else if (b == 4) { // +1
                0
            }else if (b == 5) { // +1
                1
            }else if (b == 6) { // +1
                0
            }else if (b == 7) { // +1
                1
            }else if (b == 8) { // +1
                0
            }else if (b == 9) { // +1
                1
            }else {
                12
            };
            return {return a} // +1
    }


}
