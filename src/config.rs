use directories::BaseDirs;
use serde::{Deserialize, Deserializer};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::OnceLock;

#[derive(Deserialize)]
#[serde(untagged)]
enum SwallowCriteriaValue {
    Single(Vec<String>),
    Multiple(Vec<Vec<String>>),
}

fn deserialize_swallow_criteria<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, Vec<HashSet<String>>>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = HashMap::<String, SwallowCriteriaValue>::deserialize(deserializer)?;
    Ok(raw
        .into_iter()
        .map(|(k, v)| {
            let sets = match v {
                SwallowCriteriaValue::Single(s) => {
                    vec![s.into_iter().collect::<HashSet<String>>()]
                }
                SwallowCriteriaValue::Multiple(m) => m
                    .into_iter()
                    .map(|s| s.into_iter().collect::<HashSet<String>>())
                    .collect(),
            };
            (k, sets)
        })
        .collect())
}

#[derive(Deserialize, Debug)]
pub struct WindowCommandMapping {
    pub class: Option<String>,
    pub title: Option<String>,
    pub command: Option<String>,
    pub working_directory: Option<String>,
    pub once: Option<bool>,
    pub ignored: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct TerminalCommandMapping {
    pub name: Option<String>,
    pub args: Option<String>,
    pub command: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default)]
    pub window_command_mappings: Vec<WindowCommandMapping>,
    #[serde(default)]
    pub terminal_command_mappings: Vec<TerminalCommandMapping>,
    #[serde(deserialize_with = "deserialize_swallow_criteria")]
    pub window_swallow_criteria: HashMap<String, Vec<HashSet<String>>>,
    pub terminal_allow_revive_processes: HashSet<String>,
    pub terminal_revive_commands: HashMap<String, String>,
}

pub static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn load_config() {
    let default_config = Config {
        window_command_mappings: vec![],
        terminal_command_mappings: vec![],
        window_swallow_criteria: HashMap::new(),
        terminal_allow_revive_processes: HashSet::new(),
        terminal_revive_commands: HashMap::new(),
    };

    if let Some(base_dirs) = BaseDirs::new() {
        let config_path = base_dirs.config_dir().join("i3-revive/config.json");
        let config = match fs::read_to_string(config_path) {
            Ok(content) => serde_json::from_str(&content).unwrap(),
            Err(_) => default_config,
        };
        CONFIG.set(config).unwrap();
    } else {
        CONFIG.set(default_config).unwrap();
    }
}
