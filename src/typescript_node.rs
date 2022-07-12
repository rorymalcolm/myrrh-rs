use itertools::Itertools;

use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::Hasher,
    sync::{Arc, Mutex},
};

use crate::TypeOutputCacheEntry;

#[derive(Debug)]
pub(crate) enum TypeScriptPrimativeType {
    String,
    Boolean,
    Number,
    Object,
    Array,
    Null,
}

impl TypeScriptPrimativeType {
    fn as_bytes(&self) -> &'static [u8] {
        match self {
            Self::String => b"string",
            Self::Boolean => b"boolean",
            Self::Number => b"number",
            Self::Object => b"object",
            Self::Array => b"array",
            Self::Null => b"null",
        }
    }
}

#[derive(Debug)]
pub(crate) struct TypeScriptNode {
    lookup_table: Arc<Mutex<HashMap<u64, usize>>>,
    name: Option<String>,
    nullable: bool,
    optional: bool,
    is_array: bool,
    root_node: bool,
    sub_items: Vec<TypeScriptNode>,
    type_signature: TypeScriptPrimativeType,
    hash: u64,
}

impl TypeScriptNode {
    pub(crate) fn calculate_hash(&mut self) -> u64 {
        let mut hasher = DefaultHasher::new();
        let mut hash_seen_before = HashSet::<u64>::new();
        for sub_item in &mut self.sub_items {
            hasher.write(sub_item.type_signature.as_bytes());
            hasher.write(sub_item.name.as_ref().unwrap_or(&"".to_string()).as_bytes());
            let sub_node_hash = &sub_item.calculate_hash();
            if hash_seen_before.contains(sub_node_hash) {
                continue;
            } else {
                hasher.write(&sub_node_hash.to_le_bytes());
                hash_seen_before.insert(*sub_node_hash);
            }
        }
        let hash = hasher.finish();
        let mut table = self.lookup_table.lock().unwrap();
        if !table.contains_key(&hash) {
            table.insert(hash, 1);
        } else {
            let current_value = table[&hash];
            table.insert(hash, current_value + 1);
        }
        self.hash = hash;
        hash
    }

    pub(crate) fn is_array(&self) -> bool {
        return self.is_array;
    }

    pub fn new(
        lookup_table: Arc<Mutex<HashMap<u64, usize>>>,
        type_name: TypeScriptPrimativeType,
        optional: bool,
        nullable: bool,
        is_array: bool,
        root_node: bool,
    ) -> Self {
        TypeScriptNode {
            lookup_table,
            name: None,
            nullable,
            optional,
            is_array,
            root_node,
            sub_items: Vec::new(),
            type_signature: type_name,
            hash: 0,
        }
    }

    fn newline_if_parent_not_array_node(array_node: bool) -> String {
        if !array_node {
            String::from("\n")
        } else {
            String::new()
        }
    }

    fn semicolon_if_parent_array_node(array_node: bool) -> String {
        if array_node {
            String::from(";")
        } else {
            String::new()
        }
    }

    fn space_if_parent_not_root_node(root_node: bool) -> String {
        if !root_node {
            String::from(" ")
        } else {
            String::new()
        }
    }

    fn string_is_alphanumeric(string: &str) -> bool {
        string.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    pub(crate) fn to_type_string(node: TypeScriptNode, array_node: bool) -> String {
        let mut type_output_cache = HashMap::<u64, TypeOutputCacheEntry>::new();
        let mut type_string = String::new();
        type_string.push_str("type DefaultType = ");
        type_string.push_str(&Self::to_type_string_helper(
            node,
            array_node,
            0,
            &mut type_output_cache,
        ));
        type_output_cache
            .into_iter()
            .sorted()
            .for_each(|(_, value)| {
                type_string.push_str(
                    format!("\ntype {} = {}\n", &value.type_name, &value.output).as_str(),
                );
            });
        type_string
    }

    fn to_type_string_helper(
        node: TypeScriptNode,
        parent_array_node: bool,
        indent_size: usize,
        type_output_cache: &mut HashMap<u64, TypeOutputCacheEntry>,
    ) -> String {
        let mut type_string = String::new();
        let mut indent_string = String::new();
        if !node.root_node && !parent_array_node {
            for _ in 0..indent_size {
                indent_string.push_str("  ");
            }
            type_string.push_str(&indent_string)
        }
        match node.name {
            Some(name) => {
                if Self::string_is_alphanumeric(&name.clone()) {
                    type_string.push_str(&format!("{}: ", name));
                } else {
                    type_string.push_str(&format!("\"{}\": ", name))
                }
            }
            None => (),
        }
        match node.type_signature {
            TypeScriptPrimativeType::Boolean => type_string.push_str("boolean"),
            TypeScriptPrimativeType::String => type_string.push_str("string"),
            TypeScriptPrimativeType::Number => type_string.push_str("number"),
            TypeScriptPrimativeType::Null => type_string.push_str("null"),
            TypeScriptPrimativeType::Object => {
                if type_output_cache.contains_key(&node.hash) {
                    type_string.push_str(&type_output_cache[&node.hash].type_name);
                } else {
                    let mut object_type_string = String::new();
                    object_type_string.push_str(&format!(
                        "{{{}{}",
                        &Self::newline_if_parent_not_array_node(parent_array_node),
                        &Self::space_if_parent_not_root_node(node.root_node)
                    ));
                    for o in node.sub_items {
                        object_type_string.push_str(&format!(
                            "{}{}{}",
                            TypeScriptNode::to_type_string_helper(
                                o,
                                parent_array_node,
                                indent_size + 1,
                                type_output_cache,
                            ),
                            &Self::space_if_parent_not_root_node(parent_array_node),
                            &Self::semicolon_if_parent_array_node(parent_array_node)
                        ));
                    }
                    object_type_string.push_str(&format!(
                        "{}{}}}",
                        &Self::space_if_parent_not_root_node(node.root_node),
                        indent_string
                    ));
                    let object_type_output = object_type_string.clone();
                    let lookup_table = node.lookup_table.lock().unwrap();
                    if lookup_table.contains_key(&node.hash) && lookup_table[&node.hash] > 1 {
                        let len = type_output_cache.len();
                        let type_name = format!("DefaultType_{}", len);
                        type_output_cache.insert(
                            node.hash,
                            TypeOutputCacheEntry::new(
                                type_name.clone(),
                                object_type_output.clone(),
                            ),
                        );
                        type_string.push_str(&type_name);
                    } else {
                        type_string.push_str(&object_type_string.clone())
                    }
                }
            }
            TypeScriptPrimativeType::Array => {
                let mut array_types_seen = HashSet::<String>::new();
                for a in node.sub_items {
                    let array_type = TypeScriptNode::to_type_string_helper(
                        a,
                        true,
                        indent_size + 1,
                        type_output_cache,
                    );
                    array_types_seen.insert(array_type);
                }
                let to_append = match array_types_seen.len() {
                    0 => "any".to_string(),
                    1 => format!("{}", array_types_seen.into_iter().next().unwrap()),
                    _ => {
                        format!("({})", &array_types_seen.iter().sorted().join(" | "))
                    }
                };
                type_string.push_str(&to_append);
                type_string.push_str("[]");
            }
        }
        if node.optional {
            type_string.push_str("?");
        }
        if node.nullable {
            type_string.push_str("null");
        }
        if !parent_array_node {
            type_string.push_str(";\n");
        }
        type_string
    }

    pub(crate) fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub(crate) fn with_sub_items(mut self, sub_items: Vec<TypeScriptNode>) -> Self {
        self.sub_items = sub_items;
        self
    }
}
