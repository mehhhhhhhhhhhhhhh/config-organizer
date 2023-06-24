mod environment_definitions;
mod processing;
mod variable_definitions;

use environment_definitions::EnvironmentDefinitions;
use processing::{Template, TemplateFormat};
use variable_definitions::VariableSource;

use crate::processing::Environment;
use clap::{Parser, ValueEnum};
use serde_yaml::Value;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ffi::{OsString};
use std::fs::{read_dir, File};
use std::io::{read_to_string, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::FromStr;
use std::{env, fs, io};

#[derive(Copy, Clone, Debug, ValueEnum)]
enum OutputFormat {
    CanonicalJson,
    #[deprecated]
    Yaml,
}

#[derive(Parser)]
struct Args {
    #[arg(default_value = default_input_directory().into_os_string())]
    input_directory: PathBuf,
    #[arg(default_value = default_output_directory().into_os_string())]
    output_directory: PathBuf,

    #[arg(long = "envs", default_value = default_envs_file().into_os_string())]
    environments_file_path: PathBuf,

    #[deprecated]
    #[arg(value_enum, long = "format", default_value_t = OutputFormat::CanonicalJson)]
    format: OutputFormat,
    // TODO control over verboseness
}

#[test]
fn tests_tester() {}

fn default_input_directory() -> PathBuf {
    PathBuf::from_str(".").unwrap()
}
fn default_output_directory() -> PathBuf {
    PathBuf::from_str("./environments").unwrap()
}
fn default_envs_file() -> PathBuf {
    PathBuf::from_str("./environments.yml").unwrap()
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
    } else if vec![".conf", ".env", ".txt", ".php"]
        .iter()
        .any(|ext| as_str.ends_with(ext))
    {
        TemplateFormat::Text
    } else {
        panic!(
            "Couldn't determine processing format for filename \"{as_str}\""
        )
    }
}

fn get_templates() -> Vec<Template> {
    let stuff =
        read_dir(PathBuf::from("configuration/templates")).expect("Failed to list templates");
    stuff
        .into_iter()
        .map(|template_listing| {
            let template_dir_entry = template_listing.expect("WTF");
            let filename = template_dir_entry.file_name();
            let format = determine_format(&filename);
            Template {
                format,
                source_path: template_dir_entry.path(),
            }
        })
        .collect()
}

fn write_text(content: &str, output_path: &Path) -> io::Result<()> {
    if let Some(true) = File::open(output_path)
        .ok()
        .and_then(|f| Some(read_to_string(f).ok()? == content))
    {
        eprintln!("Unchanged {output_path:?}");
        return Ok(());
    }
    eprintln!("Writing {output_path:?}");

    let mut output_file = File::create(output_path)?;
    output_file.write_all(content.as_bytes())
}

fn write_full_yaml(content: &Value, output_path: &Path) -> io::Result<()> {
    if let Some(true) = File::open(output_path).ok().and_then(|f| {
        Some(read_to_string(f).ok()? == serde_yaml::to_string(content).expect("YAML error"))
    }) {
        eprintln!("Unchanged {output_path:?}");
        return Ok(());
    }
    eprintln!("Writing {output_path:?}");

    let output_file = File::create(output_path)?;
    serde_yaml::to_writer(output_file, content)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn write_canonical_json(content: &Value, output_path: &Path) -> io::Result<()> {
    let canonical_json =
        json_canon::to_string(&serde_json::to_value(content).expect("JSON conversion error"))
            .expect("Canonical JSON error");

    if let Some(true) = File::open(output_path)
        .ok()
        .and_then(|f| Some(read_to_string(f).ok()? == (canonical_json.clone() + "\n")))
    {
        eprintln!("Unchanged {output_path:?}");
        return Ok(());
    }
    eprintln!("Writing {output_path:?}");

    // Note: this is RFC 8785 canonical json -- not the weird OLPC bullshit, which we can't use as it forbids floats.
    let mut output_file = File::create(output_path)?;
    output_file.write_all((canonical_json + "\n").as_bytes())
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    env::set_current_dir(args.input_directory)?;

    let envs_file = File::open(args.environments_file_path)?;
    let env_defs: EnvironmentDefinitions = serde_yaml::from_reader(envs_file).unwrap();
    let envs = env_defs.environments;

    let mut cache = VarDefParseCache {
        cache: Default::default(),
    };

    for (name, def) in envs {
        //println!("{}:\n  {:?}", &name, &def);

        let mut output_dir = args.output_directory.to_path_buf();
        output_dir.push(Path::new(&format!("{}/configs", &name)));
        fs::create_dir_all(&output_dir)?;

        let mut var_sources: Vec<Rc<VariableSource>> = vec![];
        for var_source_path in def.configuration.variables {
            //println!("    {}", &var_source);
            let path = format!("configuration/variables/{}.yml", &var_source_path);
            let var_source = cache.load(Path::new(&path))?;
            var_sources.push(Rc::clone(var_source));
            //println!("      {:?}", shit);
        }

        let mut combined_source: VariableSource =
            variable_definitions::combine(var_sources.iter().map(|x| x.deref()).collect());
        //eprintln!("{:?}", &combined_source.mutations.iter().map(|m| &m.filename_pattern).collect::<Vec<_>>());

        // for (k,v) in &combined_source.definitions {
        //     println!("{}: {:?}", &k, &v)
        // }
        combined_source
            .definitions
            .insert("environment/name".to_string(), Value::String(name.clone()));
        let environment = Environment {
            definitions: combined_source,
            expected_runtime_lookup_prefixes: def
                .configuration
                .external_namespaces
                .iter()
                .map(|ns| ns.to_string() + "/")
                .collect(),
        };

        for template in get_templates() {
            let filename = template.source_path.file_name().unwrap().to_str().unwrap();
            if def
                .configuration
                .excluded_files
                .iter()
                .any(|ex_fn| ex_fn == filename)
            {
                eprintln!("Skipping {}", &filename);
                continue;
            }
            let output_path = output_dir
                .join(template.source_path.file_name().unwrap().to_str().unwrap())
                .as_path()
                .to_owned();

            match template.format {
                TemplateFormat::Yaml => {
                    let result = processing::process_yaml(&template, &environment);
                    let output_fn = match args.format {
                        OutputFormat::CanonicalJson => write_canonical_json,
                        OutputFormat::Yaml => write_full_yaml,
                    };
                    output_fn(&result, &output_path)?;
                }
                TemplateFormat::Text => {
                    let result = processing::process_text(&template, &environment);
                    write_text(&result, &output_path)?;
                }
            }
        }
    }
    Ok(())
}
