use std::fs::{File, read_to_string, write};
use std::io;
use std::io::Write;
use std::iter::Map;
use std::path::PathBuf;
use serde::Serialize;
use serde_yaml::{Mapping, Sequence, Value};
use crate::variable_definitions::{Mutation, MutationAction, string_value, VariableSource};

// TODO support working in YAML but with Canonical JSON (RFC) output
#[derive(Debug)]
pub(crate) struct Environment {
    pub(crate) definitions: VariableSource,
    pub(crate) expected_runtime_lookup_prefixes: Vec<String>,
}

#[derive(Debug)]
pub(crate) enum TemplateFormat {
    Yaml, Text
}

#[derive(Debug)]
pub(crate) struct Template {
    pub(crate) format: TemplateFormat,
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
        let next = self.get_mut(path.get(0).expect(&format!("WTF, regarding path {:?}", &path))).expect(&format!("WTF, regarding missing value at {:?}", &path));
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

trait TryNavigate {
    fn try_navigate(&mut self, path: &[String]) -> Option<&mut Value>;
}
impl TryNavigate for Mapping {
    fn try_navigate(&mut self, path: &[String]) -> Option<&mut Value> {
        let next = self.get_mut(path.get(0).expect(&format!("WTF, regarding path {:?}", &path)));
        return next.and_then(|next| next.try_navigate(&path[1..]))
    }
}
impl TryNavigate for Value {
    fn try_navigate(&mut self, path: &[String]) -> Option<&mut Value> {
        if path.len() == 0 {
            return Some(self)
        }
        mapping_value(self).expect("not a mapping").try_navigate(path)
    }
}

fn apply_mutation(mutation: &MutationAction, content: &mut Value){
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
fn lookup(reference_name: &str, environment: &Environment) -> Option<Value> {
    let should_be_runtime_value = environment.expected_runtime_lookup_prefixes.iter().any(|prefix| reference_name.starts_with(prefix));
    let should_be_json = reference_name.ends_with("/json");

    match _lookup(reference_name, environment) {
        None => {
            if should_be_runtime_value {
                None
            } else {
                panic!("Couldn't find definition for {}", &reference_name)
            }
        }
        Some(val) => {
            if should_be_runtime_value {
                eprintln!("WARN: Runtime value \"{}\" was unexpectedly hardcoded.", reference_name)
            }
            if should_be_json {
                let expanded_val = expand(val, environment);
                if let Value::Mapping(m) = expanded_val {
                    Some(Value::String(canonical_json::to_string(&serde_json::to_value(m).unwrap()).unwrap()))
                } else if let Value::String(s) = expanded_val {
                    Some(Value::String(s))
                } else {
                    panic!("Received non-mapping value for /json conversion: {:?}", &expanded_val)
                }
            } else {
                Some(expand(val, environment))
            }
        }
    }
}

fn expand_string(string: String, environment: &Environment) -> Value {
    let lpos = string.find("((");
    let rpos = string.find("))");
    if let Some(lpos) = lpos {
        if let Some(rpos) = rpos {
            let reference_name = &string[lpos+2..rpos].trim();
            if lpos == 0 && rpos == (string.len()-2) {
                return lookup(reference_name, environment).unwrap_or(Value::String(string))
            } else if reference_name.find("(").is_none() {  // avoid being tripped up by regexes :grimace:
                let val = lookup(reference_name, environment);
                let str_val = match val {
                    None => {return Value::String(string)},
                    Some(Value::Number(n)) => format!("{}", n),
                    Some(Value::String(str)) => str,
                    Some(val) => panic!("Attempted to interpolate non-string value \"{}\" ({:?})", reference_name, val),
                };
                return expand_string(string[..lpos].to_string() + &str_val + &string[rpos+2..], environment)
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

pub(crate) fn process_text(template: &Template, environment: &Environment) -> String {
    let text = read_to_string(&template.source_path).unwrap();
    string_value(&expand_string(text, environment)).expect("Text template somehow expanded to a non-string value")
}

pub(crate) fn process_yaml(template: &Template, environment: &Environment) -> Value {
    let filename = template.source_path.file_name().unwrap().to_str().unwrap();
    let mut content : Value = serde_yaml::from_reader(File::open(&template.source_path).unwrap()).unwrap();

    for mutation in &environment.definitions.mutations {
        if mutation.filename_pattern == filename {
            apply_mutation(&mutation.action, &mut content);
        }
    }
    let mut content = expand(content, environment);
    postprocess_yaml(&mut content);
    content
}

fn postprocess_yaml(mut yaml_config: &mut Value) {
    if let Some(profiles) = yaml_config.try_navigate(&vec!["spring".to_string(), "profiles".to_string()]) {
        if let Value::Mapping(profiles) = profiles {
            if let Some(Value::Sequence(active_profiles)) = profiles.get("active") {
                profiles.insert(
                    Value::String("active".to_string()),
                    Value::String(
                        active_profiles.iter().map(|prof| {
                            string_value(prof).unwrap()
                        }).collect::<Vec<_>>().join(",")
                    )
                );
            }
        }
    }
}
