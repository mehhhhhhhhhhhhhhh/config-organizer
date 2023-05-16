mod environment_definitions;
mod variable_definitions;

use environment_definitions::EnvironmentDefinitions;

use std::{env, io};
use std::fs::File;
use std::path::{Path, PathBuf};
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg()]
    directory: PathBuf,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    env::set_current_dir(args.directory)?;

    let envs_file = File::open("environments.yml")?;
    let env_defs : EnvironmentDefinitions = serde_yaml::from_reader(envs_file).unwrap();
    let envs = env_defs.environments;

    for (name, def) in envs {
        println!("{}:\n  {:?}", &name, &def);
        for var_source in def.configuration.variables {
            println!("    {}", &var_source);
            let path = format!("configuration/variables/{}.yml", &var_source);
            let shit = variable_definitions::load(Path::new(&path));
            eprintln!("      {:?}", shit);
        }
    }
    Ok(())
}
