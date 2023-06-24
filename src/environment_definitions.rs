use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub(crate) struct EnvironmentDefinitions {
    pub environments: HashMap<String, EnvDef>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct EnvDef {
    pub configuration: ConfDef,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ConfDef {
    pub variables: Vec<String>,
    #[serde(default)]
    pub external_namespaces: Vec<String>,
    #[serde(default)]
    pub excluded_files: Vec<String>,
}
