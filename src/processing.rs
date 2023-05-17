use std::fs::File;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use serde_yaml::Mapping;
use crate::variable_definitions::VariableSource;

pub(crate) enum ProcessingType {
    Yaml, Text
}
pub(crate) struct Template {
    pub(crate) filename: PathBuf,
    pub(crate) processing_type: ProcessingType,
    pub(crate) source_path: PathBuf,
}

pub(crate) fn process(template: &Template, source: &VariableSource, destination: &mut dyn Write) -> io::Result<()> {
    match template.processing_type {
        ProcessingType::Yaml => {
            let content: Mapping = serde_yaml::from_reader(File::open(&template.source_path)?).unwrap();
            process_yaml(content, source, destination);
        }
        ProcessingType::Text => { panic!("Aaagh! This isn't YAML!") }
    }
    Ok(())
}

pub(crate) fn process_yaml(template: Mapping, source: &VariableSource, destination: &mut dyn Write) -> io::Result<()> {
    // TODO mutatations
    // TODO walk and expand
    serde_yaml::to_writer(destination, &template);
    Ok(())
}
