#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct TypeOutputCacheEntry {
    pub(crate) type_name: String,
    pub(crate) output: String,
}

impl TypeOutputCacheEntry {
    pub(crate) fn new(type_name: String, output: String) -> Self {
        Self { type_name, output }
    }
}
