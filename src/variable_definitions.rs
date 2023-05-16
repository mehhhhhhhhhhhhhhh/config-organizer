use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;

use serde_yaml::{Value, Mapping};

#[derive(Debug)]
pub(crate) struct VariableSource {
    pub definitions: HashMap<String, Value>,
    pub mutations: Vec<Value>,
}

fn parse_defs(input: Mapping) -> io::Result<HashMap<String, Value>> {
    let mut map : HashMap<String, Value> = HashMap::with_capacity(input.len());
    for (key, value) in input {
        if let Value::String(string) = key {
            map.insert(string, value);
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Non-string key in variable definitions"))
        }
    }
    Ok(map)
}

fn remove_mutations(input: &mut Mapping) -> io::Result<Vec<Value>> {
    let mutations_val = input.remove("mutations");
    if let None = mutations_val {
        return Ok(vec![])
    }

    if let Some(Value::Sequence(mutations_seq)) = mutations_val {
        return Ok(mutations_seq.into())
    }

    Err(io::Error::new(io::ErrorKind::InvalidData, "Mutations must be a list"))
}

pub(crate) fn load(path: &Path) -> io::Result<VariableSource> {
    let input_file = File::open(path)?;
    let mut input: Mapping = serde_yaml::from_reader(input_file).unwrap();

    let mutations = remove_mutations(&mut input)?;

    Ok(VariableSource {
        definitions: parse_defs(input)?,
        mutations: mutations,
    })
}
