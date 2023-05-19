use std::fs::File;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use serde_yaml::{Mapping, Value};
use crate::variable_definitions::{Mutation, MutationAction, VariableSource};

pub(crate) enum Format {
    Yaml, Text
}
// TODO support working in YAML but with Canonical JSON (RFC) output

pub(crate) struct Template {
    pub(crate) filename: PathBuf,
    pub(crate) format: Format,
    pub(crate) source_path: PathBuf,
}

fn navigate_mapping<'a>(mapping: &'a mut Mapping, path: &[String]) -> &'a mut Mapping {
    if path.len() == 0 {
        return mapping
    }
    let next = mapping.get_mut(path.get(0).expect("WTF")).expect("WTF");
    if let Value::Mapping(ref mut m) = next {
        return navigate_mapping(m, &path[1..])
    }
    panic!()
}

fn apply_mutation(mutation: &MutationAction, content: &mut Mapping){
    match mutation {
        MutationAction::Add(path, Value::Mapping(new_entries)) => {}
        MutationAction::Add(path, Value::Sequence(new_elems)) => {}
        MutationAction::Add(path, _) => { panic!("Add mutation is trying to add non-mapping, non-sequence values") }
        MutationAction::Remove(path) => {
            navigate_mapping(content, &path[..(path.len()-1)])
                .remove(&path[path.len()-1])
                .expect(&format!("can't remove missing {:?}", &path));
        }
        MutationAction::Replace(path, v) => {}
    }
}

pub(crate) fn process(template: &Template, source: &VariableSource, destination: &mut dyn Write) -> io::Result<()> {
    match template.format {
        Format::Yaml => {
            let content: Mapping = serde_yaml::from_reader(File::open(&template.source_path)?).unwrap();
            process_yaml(content, template.filename.to_string_lossy().to_string(), source, destination)?;
        }
        Format::Text => { panic!("Aaagh! This isn't YAML!") }
    }
    Ok(())
}

pub(crate) fn process_yaml(mut content: Mapping, filename: String, source: &VariableSource, destination: &mut dyn Write) -> io::Result<()> {
    for mutation in &source.mutations {
        if mutation.filename_pattern == filename {
            apply_mutation(&mutation.action, &mut content);
        }
    }
    // TODO walk and expand
    serde_yaml::to_writer(destination, &content);  // TODO handle errors
    Ok(())
}
