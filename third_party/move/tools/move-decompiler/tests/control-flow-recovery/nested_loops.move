//* Test cases with nested loops
module 0x99::nested_loops {
    fun nested_for_loops() {
        let y = 0;
        for (i in 0..10) {
            y = y + 1;
            for (j in i..10) {
              y = y + 1;
            };
        };
    }

    fun nested_while_loops() {
        let x = 0;
        let z = 0;
        let y;
        while (x < 3) {
            x = x + 1;
            y = 0;
            while (y < 7) {
                y = y + 1;
                z = z + 1;
            }
        };
    }

    fun nested_loop_loops () {
        let x = 0;
        let z = 0;
        let y;
        loop {
            x = x + 1;
            y = 0;
            if (x > 3)
                break;
            loop {
                y = y + 1;
                z = z + 1;
                if (y > 7)
                    break;
            };
        };
  }

    fun nested_for_while_loops() {
        let y = 0;
        for (i in 0..5) {
            y = y + 1;
            while (y < 5) {
                y = y + 10;
            };
        };
    }

    fun nested_loop_while_loops() {
        let x = 0;
        let z = 0;
        let y;
        loop {
            x = x + 1;
            y = 0;
            if (x > 3)
                break;
            while (y < 7) {
                y = y + 1;
                z = z + 1;
            }
        };
    }

    fun nested_loop_for_loops() {
        let x = 0;
        let z = 0;
        let y;
        loop {
            x = x + 1;
            y = 0;
            if (x > 3)
                break;
            for (i in 0..5) {
                y = y + 1;
                z = z + 1;
            }
        };
    }

    fun three_layer_for_loops(){
        let y = 0;
        for (i in 0..10) {
            y = y + 1;
            for (j in i..10) {
              y = y + 1;
              for (k in j..10) {
                y = y + 1;
              };
            };
        };
    }

    fun three_layer_while_loops(){
        let y = 0;
        let i = 0;
        while(i < 10) {
            y = y + 1;
            let j = i;
            i = i + 1;
            while(j < 10) {
              y = y + 1;
              let k = j;
              j = j + 1;
              while(k < 10) {
                y = y + 1;
                k = k + 1;
              };
            };
        };
    }

    fun three_layer_loop_loops(){
        let y = 0;
        let i = 0;
        loop {
            y = y + 1;
            let j = i;
            if (i > 10)
                break;
            i = i + 1;
            loop {
              y = y + 1;
              let k = j;
              if (j >  10)
                break;
              j = j + 1;
              loop {
                y = y + 1;
                if (k > 10)
                    break;
                k = k + 1;
              };
            };
        };
    }

    fun nested_for_while_loop_loops(){
        let y = 0;
        let i = 0;
        for(i in 0..5) {
            y = y + 1;
            let j = i;
            while(j < 10) {
              y = y + 1;
              let k = j;
              j = j + 1;
              loop {
                y = y + 1;
                if (k > 10)
                    break;
                k = k + 1;
              };
            };
        };
    }
}
