use std::collections::HashMap;
use serde::{
    Deserialize,
};

#[derive(Deserialize, Debug)]
pub(crate) struct EnvironmentDefinitions {
    pub environments: HashMap<String, EnvDef>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct EnvDef {
    pub configuration: ConfDef,
    mocks: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ConfDef {
    pub variables: Vec<String>,
    #[serde(default)]
    pub external_namespaces: Vec<String>,
    #[serde(default)]
    pub excluded_files: Vec<String>,
}
