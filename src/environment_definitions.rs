use std::collections::HashMap;
use serde::{Deserialize};

#[derive(Deserialize, Debug)]
pub(crate) struct EnvironmentDefinitions {
    pub environments: HashMap<String, EnvDef>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="lowercase")]
enum FrontendType {
    Separate,
    Integrated,
}

fn separatev() -> FrontendType { FrontendType::Separate }
fn falsev() -> bool {false}

#[derive(Deserialize, Debug)]
pub(crate) struct EnvDef {
    #[serde(default="separatev")]
    frontend: FrontendType,
    #[serde(default="falsev")]
    reduced_memory: bool,
    #[serde(default="falsev")]
    debug_ports: bool,
    pub configuration: ConfDef,
    mocks: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ConfDef {
    pub variables: Vec<String>,
}
