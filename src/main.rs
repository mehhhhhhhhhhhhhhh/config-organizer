mod environment_definitions;
mod variable_definitions;
mod processing;

use environment_definitions::EnvironmentDefinitions;
use processing::{Template, ProcessingType};
use variable_definitions::VariableSource;

use std::{env, fs, io};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use clap::Parser;

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

fn get_templates() -> Vec<Template> {
    vec![Template {
        filename: PathBuf::from("auth-service.yml"),
        processing_type: ProcessingType::Yaml,
        source_path: PathBuf::from("configuration/templates/auth-service.yml"),
    }]
    // TODO return all the others too...
    // TODO make selection based on environment's required files somehow?
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

        let uber_source : VariableSource = variable_definitions::combine(var_sources.iter().map(|x| x.deref()).collect());

        for template in get_templates() {
            processing::process(&template, &uber_source, &mut File::create(output_dir.join(&template.filename))?)?;
        }
    }
    Ok(())
}
