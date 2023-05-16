mod environment_definitions;
mod variable_definitions;

use environment_definitions::EnvironmentDefinitions;

use std::{env, io};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use clap::Parser;
use crate::variable_definitions::VariableSource;

#[derive(Parser)]
struct Args {
    #[arg()]
    directory: PathBuf,
}

struct VarDefParseCache {
    cache: HashMap<PathBuf, VariableSource>,
}
impl VarDefParseCache {
    fn load(&mut self, path: &Path) -> io::Result<&VariableSource> {
        match self.cache.entry(path.to_path_buf()) {
            Entry::Occupied(v) => Ok(v.into_mut()),
            Entry::Vacant(v) => {
                eprintln!("        loading!...");
                Ok(v.insert(variable_definitions::load(path)?))
            }
        }
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    env::set_current_dir(args.directory)?;

    let envs_file = File::open("environments.yml")?;
    let env_defs : EnvironmentDefinitions = serde_yaml::from_reader(envs_file).unwrap();
    let envs = env_defs.environments;

    let mut cache = VarDefParseCache { cache: Default::default() };

    for (name, def) in envs {
        println!("{}:\n  {:?}", &name, &def);
        for var_source in def.configuration.variables {
            println!("    {}", &var_source);
            let path = format!("configuration/variables/{}.yml", &var_source);
            let shit = cache.load(Path::new(&path));
            eprintln!("      {:?}", shit);
        }
    }
    Ok(())
}
