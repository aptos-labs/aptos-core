// Copyright © Aptos Foundation

pub struct UnionFind {
    parent_of: Vec<usize>,
    depth_of: Vec<usize>,
}

impl UnionFind {
    pub fn new(num_participants: usize) -> Self {
        Self {
            parent_of: (0..num_participants).collect(),
            depth_of: vec![0; num_participants],
        }
    }

    pub fn find(&mut self, a: usize) -> usize {
        let pa = self.parent_of[a];
        if pa == a {
            return a;
        }
        let ppa = self.find(pa);
        self.parent_of[a] = ppa;
        return ppa;
    }

    pub fn union(&mut self, x: usize, y: usize) {
        let px = self.find(x);
        let py = self.find(y);
        if px==py {
            return;
        }

        if self.depth_of[px] < self.depth_of[py] {
            self.parent_of[px] = py;
        } else if self.depth_of[px] > self.depth_of[py]{
            self.parent_of[px] = py;
        } else {
            self.parent_of[px] = py;
            self.depth_of[py] += 1;
        }
    }
}

#[test]
fn test_union_find() {
    let mut uf = UnionFind::new(5);
    uf.union(0, 3);
    assert_eq!(uf.find(0), uf.find(3));
    assert_ne!(uf.find(1), uf.find(4));
    uf.union(1, 4);
    assert_ne!(uf.find(0), uf.find(4));
    uf.union(3, 1);
    assert_eq!(uf.find(0), uf.find(4));
    assert_ne!(uf.find(2), uf.find(4));
    assert_ne!(uf.find(2), uf.find(3));
    assert_ne!(uf.find(2), uf.find(1));
    assert_ne!(uf.find(2), uf.find(0));
}
