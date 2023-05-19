#![feature(let_chains)]

mod environment_definitions;
mod variable_definitions;
mod processing;

use environment_definitions::EnvironmentDefinitions;
use processing::{Template, Format};
use variable_definitions::VariableSource;

use std::{env, fs, io};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs::{File, read_dir};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use clap::Parser;
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

fn determine_format(filename: &OsString) -> Format {
    let as_str = filename.to_string_lossy();
    if as_str.ends_with(".yml") {
        Format::Yaml
    } else if vec![".conf", ".env", ".txt", ".php"].iter().any(|ext| as_str.ends_with(ext)) {
        Format::Text
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
            filename: filename.into(),
            format: format,
            source_path: template_dir_entry.path(),
        }
    }).collect();
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

        let output_dir = Path::new(&format!("envs2/{}/configs", &name)).to_path_buf();
        fs::create_dir_all(&output_dir)?;
        //println!("    {:?}", &output_dir);

        let mut var_sources : Vec<Rc<VariableSource>> = vec![];
        for var_source_path in def.configuration.variables {
            //println!("    {}", &var_source);
            let path = format!("configuration/variables/{}.yml", &var_source_path);
            let var_source = cache.load(Path::new(&path))?;
            var_sources.push(Rc::clone(var_source));
            //println!("      {:?}", shit);
        }

        let combined_source : VariableSource = variable_definitions::combine(var_sources.iter().map(|x| x.deref()).collect());
        //eprintln!("{:?}", &combined_source.mutations.iter().map(|m| &m.filename_pattern).collect::<Vec<_>>());

        // for (k,v) in &combined_source.definitions {
        //     println!("{}: {:?}", &k, &v)
        // }
        let environment = Environment {
            definitions: combined_source,
            expected_runtime_lookup_prefixes: def.configuration.external_namespaces.iter().map(|ns| ns.to_string()+"/").collect(),
        };

        for template in get_templates() {
            processing::process(&template, &environment, &mut File::create(output_dir.join(&template.filename))?)?;
        }
    }
    Ok(())
}
