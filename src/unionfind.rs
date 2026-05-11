//! Weighted union-find with path compression.

use std::fmt;

use crate::Id;

#[derive(Clone, Default)]
pub struct UnionFind {
    parents: Vec<Id>,
    sizes: Vec<u32>,
}

impl UnionFind {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.parents.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.parents.is_empty()
    }

    pub fn make_set(&mut self) -> Id {
        let id = Id::from(self.parents.len());
        self.parents.push(id);
        self.sizes.push(1);
        id
    }

    #[must_use]
    pub fn find(&self, mut id: Id) -> Id {
        while self.parents[id.index()] != id {
            id = self.parents[id.index()];
        }
        id
    }

    pub fn find_mut(&mut self, mut id: Id) -> Id {
        let mut root = id;
        while self.parents[root.index()] != root {
            root = self.parents[root.index()];
        }
        while self.parents[id.index()] != root {
            let next = self.parents[id.index()];
            self.parents[id.index()] = root;
            id = next;
        }
        root
    }

    pub fn union(&mut self, a: Id, b: Id) -> Id {
        let a = self.find_mut(a);
        let b = self.find_mut(b);
        if a == b {
            return a;
        }
        let (root, child) = if self.sizes[a.index()] >= self.sizes[b.index()] {
            (a, b)
        } else {
            (b, a)
        };
        self.parents[child.index()] = root;
        self.sizes[root.index()] += self.sizes[child.index()];
        root
    }

    #[must_use]
    pub fn equiv(&self, a: Id, b: Id) -> bool {
        self.find(a) == self.find(b)
    }
}

impl fmt::Debug for UnionFind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UnionFind({} elements)", self.parents.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_set_is_self_root() {
        let mut uf = UnionFind::new();
        let a = uf.make_set();
        let b = uf.make_set();
        assert_eq!(uf.find(a), a);
        assert_eq!(uf.find(b), b);
        assert!(!uf.equiv(a, b));
    }

    #[test]
    fn union_merges() {
        let mut uf = UnionFind::new();
        let a = uf.make_set();
        let b = uf.make_set();
        let c = uf.make_set();
        let r = uf.union(a, b);
        assert!(uf.equiv(a, b));
        assert!(!uf.equiv(a, c));
        assert_eq!(uf.find(a), r);
        assert_eq!(uf.find(b), r);
    }

    #[test]
    fn transitive_closure() {
        let mut uf = UnionFind::new();
        let ids: Vec<_> = (0..5).map(|_| uf.make_set()).collect();
        uf.union(ids[0], ids[1]);
        uf.union(ids[2], ids[3]);
        uf.union(ids[1], ids[2]);
        for w in ids.windows(2).take(3) {
            assert!(uf.equiv(w[0], w[1]));
        }
        assert!(!uf.equiv(ids[0], ids[4]));
    }

    #[test]
    fn path_compression_actually_compresses() {
        let mut uf = UnionFind::new();
        let ids: Vec<_> = (0..6).map(|_| uf.make_set()).collect();
        for w in ids.windows(2) {
            uf.union(w[0], w[1]);
        }
        let root = uf.find_mut(ids[0]);
        for &id in &ids {
            uf.find_mut(id);
        }
        for &id in &ids {
            assert_eq!(uf.parents[id.index()], root);
        }
    }
}
