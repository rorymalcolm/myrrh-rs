use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::Hasher,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub struct MerkleTree<T> {
    lookup_up_table: HashmapWrapper,
    root: Option<Leaf<T>>,
}

#[derive(Debug, Clone)]
pub struct Leaf<T> {
    value: T,
    hash: u64,
    leaves: Vec<Leaf<T>>,
}

#[derive(Debug)]
pub struct HashmapWrapper(Mutex<HashMap<u64, u64>>);
impl HashmapWrapper {
    pub fn new() -> Self {
        HashmapWrapper(Mutex::new(HashMap::new()))
    }

    pub fn contains_key(&self, id: u64) -> bool {
        self.0.lock().unwrap().contains_key(&id)
    }

    pub fn get(&self, id: u64) -> Option<u64> {
        self.0.lock().unwrap().get(&id).cloned()
    }

    pub fn insert(&mut self, id: u64, value: u64) {
        self.0.lock().unwrap().insert(id, value);
    }

    pub fn len(&self) -> usize {
        self.0.lock().unwrap().len()
    }
}

impl<T: std::hash::Hash + Clone + core::fmt::Debug> Leaf<T> {
    pub fn new(value: T, merkle_tree: &Arc<Mutex<&mut MerkleTree<T>>>) -> Self {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        let hash = hasher.finish();
        if merkle_tree
            .lock()
            .unwrap()
            .lookup_up_table
            .contains_key(hash)
        {
            let new_value = merkle_tree
                .lock()
                .unwrap()
                .lookup_up_table
                .get(hash)
                .unwrap()
                + 1;

            merkle_tree
                .lock()
                .unwrap()
                .lookup_up_table
                .insert(hash, new_value);
        } else {
            let lookup_table = &mut merkle_tree.lock().unwrap().lookup_up_table;
            lookup_table.insert(hash, 1);
        }

        Leaf {
            value,
            hash,
            leaves: vec![],
        }
    }

    pub fn get_leaves(&self) -> Vec<Leaf<T>> {
        self.leaves.clone()
    }

    pub fn add_leaf(&mut self, leaf: T, merkle_tree: &Arc<Mutex<&mut MerkleTree<T>>>) {
        self.leaves.push(Self::new(leaf, merkle_tree));
        self.hash = Self::compute_tree_hash(&self);
    }

    fn compute_tree_hash(leaf: &Leaf<T>) -> u64 {
        let mut hasher = DefaultHasher::new();
        leaf.value.hash(&mut hasher);
        for leaf in leaf.leaves.clone() {
            Self::compute_hash_tree_helper(&leaf.clone(), &mut hasher);
        }
        hasher.finish()
    }

    fn compute_hash_tree_helper(leaf: &Leaf<T>, hasher: &mut DefaultHasher) {
        leaf.value.hash(hasher);
        for leaf in leaf.leaves.clone() {
            Self::compute_hash_tree_helper(&leaf.clone(), hasher);
        }
    }
}

impl<T: std::hash::Hash + Clone + core::fmt::Debug> MerkleTree<T> {
    pub fn new() -> Self {
        let val = MerkleTree {
            lookup_up_table: HashmapWrapper::new(),
            root: None,
        };
        val
    }

    pub fn with_root(&mut self, root: T) -> &mut Self {
        self.root = Some(Leaf::new(root, &Arc::new(Mutex::new(self))));
        self
    }

    pub fn get_root(&mut self) -> Arc<Mutex<Leaf<T>>> {
        if self.root.is_none() {
            panic!("root is None");
        }
        Arc::new(Mutex::new(self.root.clone().unwrap()))
    }

    pub fn compute_root_hash(param: T) -> u64 {
        let mut hasher = DefaultHasher::new();
        param.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::merkle_tree::Leaf;

    use super::MerkleTree;

    #[test]
    fn initialises() {
        let _ = MerkleTree::new().with_root("root".to_string());
    }

    #[test]
    fn multi_node() {
        let mut merkle_tree_init = MerkleTree::new();
        let merkle_tree = merkle_tree_init.with_root("root".to_string());
        let root: Arc<Mutex<Leaf<std::string::String>>> = merkle_tree.get_root();
        root.try_lock()
            .unwrap()
            .add_leaf("right".to_string(), &Arc::new(Mutex::new(merkle_tree)));
        assert_eq!(merkle_tree.lookup_up_table.len(), 2)
    }

    #[test]
    fn hash_changes_on_insert(){
        let mut merkle_tree_init = MerkleTree::new();
        let merkle_tree = merkle_tree_init.with_root("root".to_string());
        let root: Arc<Mutex<Leaf<std::string::String>>> = merkle_tree.get_root();
        let hash = root.try_lock().unwrap().hash.clone();
        root.try_lock()
            .unwrap()
            .add_leaf("right".to_string(), &Arc::new(Mutex::new(merkle_tree)));
        assert_ne!(hash, merkle_tree.get_root().lock().unwrap().hash)
    }
}
