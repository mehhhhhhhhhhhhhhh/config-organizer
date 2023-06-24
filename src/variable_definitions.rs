
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;

use serde_yaml::{Mapping, Value};

type ValuePath = Vec<String>;

#[derive(Debug, Clone)]
pub(crate) struct Mutation {
    pub(crate) filename_pattern: String, // TODO is it actually just an exact match? is an exact match sufficient?
    pub(crate) action: MutationAction,
}
#[derive(Debug, Clone)]
pub(crate) enum MutationAction {
    Add(ValuePath, Value),
    Remove(ValuePath),
    Replace(ValuePath, Value),
}

#[derive(Debug)]
pub(crate) struct VariableSource {
    pub definitions: HashMap<String, Value>,
    pub mutations: Vec<Mutation>,
}

fn parse_defs(input: Mapping) -> io::Result<HashMap<String, Value>> {
    let mut map: HashMap<String, Value> = HashMap::with_capacity(input.len());
    for (key, value) in input {
        if let Value::String(string) = key {
            map.insert(string, value);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Non-string key in variable definitions",
            ));
        }
    }
    Ok(map)
}

pub(crate) fn string_value(input: &Value) -> Option<String> {
    if let Value::String(val) = input {
        Some(val.to_string())
    } else {
        None
    }
}

fn _parse_value_path(nested_input: &Value) -> Option<Vec<String>> {
    if let Value::Mapping(nested_input) = nested_input {
        if nested_input.len() != 1 {
            return None;
        }
        let (key, more) = nested_input.iter().next()?;
        let key = string_value(key)?;

        _parse_value_path(more).map(|mut vec| {
            vec.push(key.clone());
            vec
        })
    } else if let Value::Null = nested_input {
        return Some(vec![]);
    } else {
        return None;
    }
}

fn parse_value_path(nested_input: &Value) -> Option<Vec<String>> {
    _parse_value_path(nested_input).map(|mut vec| {
        vec.reverse();
        vec
    })
}

fn _parse_mutation(input: &Value) -> Option<Mutation> {
    if let Value::Mapping(input) = input {
        if input.len() != 1 {
            return None;
        }
        let (filename_pattern, action_yaml) = input.iter().next()?;
        let filename_pattern = string_value(filename_pattern)?;

        let action = if let Some(remove_path) = action_yaml.get("remove") {
            MutationAction::Remove(parse_value_path(remove_path)?)
        } else if let Some(add_path) = action_yaml.get("add") {
            MutationAction::Add(
                parse_value_path(add_path)?,
                action_yaml.get("values")?.clone(),
            )
        } else if let Some(replace_path) = action_yaml.get("replace") {
            MutationAction::Replace(
                parse_value_path(replace_path)?,
                action_yaml.get("value")?.clone(),
            )
        } else {
            return None;
        };

        let val = Mutation {
            filename_pattern,
            action,
        };

        Some(val)
    } else {
        None
    }
}

fn parse_mutation(input: &Value) -> io::Result<Mutation> {
    _parse_mutation(input).ok_or(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("Dodgy mutation syntax: {input:?}"),
    ))
}

fn remove_mutations(input: &mut Mapping) -> io::Result<Vec<Mutation>> {
    let mutations_val = input.remove("mutations");
    if mutations_val.is_none() {
        return Ok(vec![]);
    }

    if let Some(Value::Sequence(mutations_seq)) = mutations_val {
        let iter = mutations_seq.iter();
        let map = iter.map(parse_mutation);
        return map.collect::<io::Result<Vec<Mutation>>>();
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "Mutations must be a list",
    ))
}

pub(crate) fn load(path: &Path) -> io::Result<VariableSource> {
    let input_file = File::open(path)?;
    let mut input: Mapping = serde_yaml::from_reader(input_file).unwrap();

    let mutations = remove_mutations(&mut input)?;

    Ok(VariableSource {
        definitions: parse_defs(input)?,
        mutations,
    })
}

pub(crate) fn combine(sources: Vec<&VariableSource>) -> VariableSource {
    let mut all_defs: HashMap<String, Value> = HashMap::new();
    for source in sources.iter() {
        all_defs.extend(
            source
                .definitions
                .iter()
                .map(|(k, v)| (k.clone(), v.clone())),
        );
    }
    VariableSource {
        definitions: all_defs,
        mutations: sources.iter().flat_map(|s| s.mutations.clone()).collect(),
    }
}
