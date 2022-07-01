use std::{hash::Hasher, collections::{hash_map::DefaultHasher, HashMap}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleTree<T> {
    lookup_up_table: HashMap<u64, u64>,
    root: Leaf<T>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leaf<T> {
    value: T,
    hash: u64,
    leaves: Vec<Leaf<T>>
}

impl<T: std::hash::Hash + Clone> MerkleTree<T> {
    pub fn new(root: T) -> Self {
        let mut val = MerkleTree {
            lookup_up_table: HashMap::new(),
            root: Self::new_leaf(root.clone()),
        };
        val.lookup_up_table.insert(Self::compute_root_hash(root.clone()), 1);
        val
    }

    fn new_leaf(value: T) -> Leaf<T> {
        Leaf {
            value: value.clone(),
            hash: Self::compute_root_hash(value.clone()),
            leaves: vec![],
        }
    }

    pub fn add_leaf(&mut self, leaf: T) {
        self.root.leaves.push(Self::new_leaf(leaf));
        self.root.hash = Self::compute_tree_hash(self.root.clone());
        if self.lookup_up_table.contains_key(&self.root.hash) {
            self.lookup_up_table
                .insert(self.root.hash, &self.lookup_up_table[&self.root.hash] + 1);
        } else {
            self.lookup_up_table.insert(self.root.hash, 1);
        }
    }

    pub fn get_root(&self) -> Leaf<T> {
        self.root.clone()
    }

    pub fn compute_root_hash(param: T) -> u64 {
        let mut hasher = DefaultHasher::new();
        param.hash(&mut hasher);
        hasher.finish()
    }

    fn compute_tree_hash(leaf: Leaf<T>) -> u64{
        let mut hasher = DefaultHasher::new();
        leaf.value.hash(&mut hasher);
        for leaf in leaf.leaves {
            Self::compute_hash_tree_helper(leaf, &mut hasher);
        }
        hasher.finish()
    }

    fn compute_hash_tree_helper(leaf: Leaf<T>, hasher: &mut DefaultHasher) {
        leaf.value.hash(hasher);
        for leaf in leaf.leaves {
            Self::compute_hash_tree_helper(leaf, hasher);
        }
    }
}
