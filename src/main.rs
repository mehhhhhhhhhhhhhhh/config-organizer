mod environment_definitions;
mod variable_definitions;
mod processing;

use environment_definitions::EnvironmentDefinitions;
use processing::{Template, TemplateFormat};
use variable_definitions::VariableSource;

use std::{env, fs, io};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs::{File, read_dir};
use std::io::{read_to_string, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use clap::Parser;
use serde_yaml::Value;
use crate::processing::Environment;

#[derive(Parser)]
struct Args {
    #[arg()]
    directory: PathBuf,
}

struct VarDefParseCache {
    cache: HashMap<PathBuf, Rc<VariableSource>>,
}
impl VarDefParseCache {
    fn load(&mut self, path: &Path) -> io::Result<&Rc<VariableSource>> {
        match self.cache.entry(path.to_path_buf()) {
            Entry::Occupied(v) => Ok(v.into_mut()),
            Entry::Vacant(v) => {
                //println!("        loading {:?}!...", path);
                Ok(v.insert(Rc::new(variable_definitions::load(path)?)))
            }
        }
    }
}

fn determine_format(filename: &OsString) -> TemplateFormat {
    let as_str = filename.to_string_lossy();
    if as_str.ends_with(".yml") {
        TemplateFormat::Yaml
    } else if vec![".conf", ".env", ".txt", ".php"].iter().any(|ext| as_str.ends_with(ext)) {
        TemplateFormat::Text
    } else {
        panic!("Couldn't determine processing format for filename \"{}\"", as_str)
    }
}

fn get_templates() -> Vec<Template> {
    let stuff = read_dir(PathBuf::from("configuration/templates")).expect("Failed to list templates");
    return stuff.into_iter().map(|template_listing| {
        let template_dir_entry = template_listing.expect("WTF");
        let filename = template_dir_entry.file_name();
        let format = determine_format(&filename);
        Template {
            format: format,
            source_path: template_dir_entry.path(),
        }
    }).collect();
}

fn write_text(content: &str, output_path: &Path) -> io::Result<()> {
    if let Some(true) = File::open(output_path).ok().and_then(|mut f|{
        Some(read_to_string(f).ok()? == content)
    }) {
        eprintln!("File {:?} is unchanged", output_path);
        return Ok(())
    }
    let mut output_file = File::create(output_path)?;
    output_file.write_all(content.as_bytes())
}

fn write_full_yaml(content: &Value, output_path: &Path) -> io::Result<()> {
    if let Some(true) = File::open(output_path).ok().and_then(|mut f|{
        Some(read_to_string(f).ok()? == serde_yaml::to_string(content).expect("YAML error"))
    }) {
        eprintln!("File {:?} is unchanged", output_path);
        return Ok(())
    }
    let mut output_file = File::create(output_path)?;
    serde_yaml::to_writer(output_file, content).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn write_canonical_json(content: &Value, output_path: &Path) -> io::Result<()> {
    let canonical_json = json_canon::to_string(&serde_json::to_value(content).expect("JSON conversion error")).expect("Canonical JSON error");

    if let Some(true) = File::open(output_path).ok().and_then(|mut f|{
        Some(read_to_string(f).ok()? == (canonical_json.clone() + "\n"))
    }) {
        eprintln!("File {:?} is unchanged", output_path);
        return Ok(())
    }

    // Note: this is RFC 8785 canonical json -- not the weird OLPC bullshit, which we can't use as it forbids floats.
    let mut output_file = File::create(output_path)?;
    output_file.write_all((canonical_json + "\n").as_bytes())
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    env::set_current_dir(args.directory)?;

    let envs_file = File::open("environments.yml")?;
    let env_defs : EnvironmentDefinitions = serde_yaml::from_reader(envs_file).unwrap();
    let envs = env_defs.environments;

    let mut cache = VarDefParseCache { cache: Default::default() };

    for (name, def) in envs {
        //println!("{}:\n  {:?}", &name, &def);

        let main_output_dir = Path::new(&format!("envs2/{}/configs", &name)).to_path_buf();
        let canonical_output_dir = Path::new(&format!("envs2c/{}/configs", &name)).to_path_buf();
        println!("Writing config to {:?} and {:?}...", main_output_dir, canonical_output_dir);
        fs::create_dir_all(&main_output_dir)?;
        fs::create_dir_all(&canonical_output_dir)?;
        //println!("    {:?}", &output_dir);

        let mut var_sources : Vec<Rc<VariableSource>> = vec![];
        for var_source_path in def.configuration.variables {
            //println!("    {}", &var_source);
            let path = format!("configuration/variables/{}.yml", &var_source_path);
            let var_source = cache.load(Path::new(&path))?;
            var_sources.push(Rc::clone(var_source));
            //println!("      {:?}", shit);
        }

        let mut combined_source : VariableSource = variable_definitions::combine(var_sources.iter().map(|x| x.deref()).collect());
        //eprintln!("{:?}", &combined_source.mutations.iter().map(|m| &m.filename_pattern).collect::<Vec<_>>());

        // for (k,v) in &combined_source.definitions {
        //     println!("{}: {:?}", &k, &v)
        // }
        combined_source.definitions.insert("environment/name".to_string(), Value::String(name.clone()));
        let environment = Environment {
            definitions: combined_source,
            expected_runtime_lookup_prefixes: def.configuration.external_namespaces.iter().map(|ns| ns.to_string()+"/").collect(),
        };

        for template in get_templates() {
            let filename = template.source_path.file_name().unwrap().to_str().unwrap();
            if def.configuration.excluded_files.iter().any(|ex_fn| ex_fn==filename) {
                eprintln!("Skipping {}", &filename);
                continue
            }
            let main_output_path = main_output_dir.join(&filename);
            let canonical_output_path = canonical_output_dir.join(&template.source_path.file_name().unwrap().to_str().unwrap());

            match (template.format) {
                TemplateFormat::Yaml => {
                    let result = processing::process_yaml(&template, &environment);
                    write_full_yaml(&result, main_output_path.as_path())?;
                    write_canonical_json(&result, canonical_output_path.as_path())?;
                }
                TemplateFormat::Text => {
                    let result = processing::process_text(&template, &environment);
                    write_text(&result, main_output_path.as_path())?;
                    write_text(&result, canonical_output_path.as_path())?;
                }
            }
        }
    }
    Ok(())
}
