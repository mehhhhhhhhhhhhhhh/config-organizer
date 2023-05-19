use std::fs::File;
use std::io;
use std::io::Write;
use std::iter::Map;
use std::path::PathBuf;
use serde_yaml::{Mapping, Sequence, Value};
use crate::variable_definitions::{Mutation, MutationAction, string_value, VariableSource};

#[derive(Debug)]
pub(crate) enum Format {
    Yaml, Text
}
// TODO support working in YAML but with Canonical JSON (RFC) output
#[derive(Debug)]
pub(crate) struct Environment {
    pub(crate) definitions: VariableSource,
    pub(crate) expected_runtime_lookup_prefixes: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct Template {
    pub(crate) filename: PathBuf,
    pub(crate) format: Format,
    pub(crate) source_path: PathBuf,
}

fn mapping_value(val: &mut Value) -> Option<&mut Mapping> {
    if let Value::Mapping(ref mut m) = val {
        return Some(m)
    }
    return None
}
fn sequence_value(val: &mut Value) -> Option<&mut Sequence> {
    if let Value::Sequence(ref mut s) = val {
        return Some(s)
    }
    return None
}

trait Navigate {
    fn navigate(&mut self, path: &[String]) -> &mut Value;
}
impl Navigate for Mapping {
    fn navigate(&mut self, path: &[String]) -> &mut Value {
        let next = self.get_mut(path.get(0).expect("WTF")).expect("WTF");
        return next.navigate(&path[1..])
    }
}
impl Navigate for Value {
    fn navigate(&mut self, path: &[String]) -> &mut Value {
        if path.len() == 0 {
            return self
        }
        mapping_value(self).expect("not a mapping").navigate(path)
    }
}

fn apply_mutation(mutation: &MutationAction, content: &mut Mapping){
    match mutation {
        MutationAction::Add(path, Value::Mapping(new_entries)) => {
            let current = mapping_value(content.navigate(&path)).expect("urm");
            for (k, v) in new_entries.iter() {
                let old_val = current.insert(k.clone(), v.clone());
                if old_val.is_some() { panic!("Already had value at {:?}", path) }
            }
        }
        MutationAction::Add(path, Value::Sequence(new_elems)) => {
            let current = sequence_value(content.navigate(&path)).expect("urm");
            for v in new_elems.iter() {
                current.push(v.clone());
            }
        }
        MutationAction::Add(path, _) => { panic!("Add mutation is trying to add non-mapping, non-sequence values") }
        MutationAction::Remove(path) => {
            mapping_value(content.navigate(&path[..(path.len()-1)]))
                .expect("not a mapping")
                .remove(&path[path.len()-1])
                .expect(&format!("can't remove missing {:?}", &path));
        }
        MutationAction::Replace(path, v) => {
            let current = mapping_value(content.navigate(&path[..(path.len()-1)]))
                .expect("not a mapping");
            let old_val = current.insert(Value::String(path[path.len()-1].to_string()), v.clone());
            if old_val.is_none() {
                panic!("Value to replace at {:?} did not exist", &path)
            }
        }
    }
}

fn _lookup(reference_name: &str, environment: &Environment) -> Option<Value> {
    let maybe = environment.definitions.definitions.get(reference_name);
    match maybe {
        None => {
            let last_slash = reference_name[..reference_name.len()-2].rfind("/");
            match last_slash {
                None => None,
                Some(split_pos) => {
                    _lookup(&(reference_name[..split_pos].to_string() + "/*"), environment)
                },
            }
        },
        Some(value) => Some(value.clone())
    }
}
fn lookup(reference_name: &str, environment: &Environment) -> Value {
    let should_be_runtime_value = environment.expected_runtime_lookup_prefixes.iter().any(|prefix| reference_name.starts_with(prefix));

    match _lookup(reference_name, environment) {
        None => {
            if should_be_runtime_value {
                Value::String("(( ".to_owned() + reference_name + " ))")
            } else {
                panic!("Couldn't find definition for {}", &reference_name)
            }
        }
        Some(val) => {
            if should_be_runtime_value {
                eprintln!("WARN: Runtime value \"{}\" was unexpectedly hardcoded.", reference_name)
            }
            val
        }
    }
}


fn expand_string(string: String, environment: &Environment) -> Value {
    let lpos = string.find("((");
    let rpos = string.rfind("))");
    if let Some(lpos) = lpos {
        if let Some(rpos) = rpos {
            let reference_name = &string[lpos+2..rpos].trim();
            if lpos == 0 && rpos == (string.len()-2) {
                return lookup(reference_name, environment)
            }
        }
    }
    Value::String(string)
}
fn expand(mut content: Value, environment: &Environment) -> Value {
    match content {
        Value::Null => Value::Null,
        Value::Bool(a) => Value::Bool(a),
        Value::Number(a) => Value::Number(a),
        Value::String(str) => expand_string(str, environment),
        Value::Sequence(seq) => Value::Sequence(seq.into_iter().map(|v| expand(v, environment)).collect()),
        Value::Mapping(map) => {
            let mut stuff = map.into_iter().collect::<Vec<_>>();
            stuff.sort_by_key(|(k,v)| string_value(k));
            let stuff = stuff.into_iter().map(|(k,v)| (k, expand(v, environment))).collect();
            Value::Mapping(stuff)
        },
        Value::Tagged(_) => { panic!("what the fuck is this?") }
    }
}

pub(crate) fn process(template: &Template, environment: &Environment, destination: &mut dyn Write) -> io::Result<()> {
    match template.format {
        Format::Yaml => {
            let content: Value = serde_yaml::from_reader(File::open(&template.source_path)?).unwrap();
            process_yaml(content, template.filename.to_string_lossy().to_string(), environment, destination)?;
        }
        Format::Text => {
            //panic!("Aaagh! This isn't YAML!")
                // TODO do something proper
        }
    }
    Ok(())
}

fn process_yaml(mut content: Value, filename: String, environment: &Environment, destination: &mut dyn Write) -> io::Result<()> {
    for mutation in &environment.definitions.mutations {
        if mutation.filename_pattern == filename {
            apply_mutation(&mutation.action, mapping_value(&mut content).expect("not a mapping"));
        }
    }
    let content = expand(content, environment);
    serde_yaml::to_writer(destination, &content);  // TODO handle errors
    Ok(())
}
