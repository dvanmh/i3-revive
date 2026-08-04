use directories::BaseDirs;
use regex::escape;
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

/// How a captured property value is turned into a swallow regex.
#[derive(Debug)]
pub enum MatchMode {
    /// `^<value>$` — match the full value (current i3-revive behavior).
    Exact,
    /// `title^4` — match windows whose value starts with the first `n`
    /// **characters** (not bytes) of the saved value.
    Prefix(usize),
    /// `class$6` — match windows whose value ends with the last `n`
    /// **characters** (not bytes) of the saved value.
    Suffix(usize),
}

/// Parse a criterion entry like `"title^4"` / `"class$6"` / `"instance"`
/// into a property name and a [`MatchMode`].
fn parse_criterion(s: &str) -> Result<(String, MatchMode), String> {
    if let Some(pos) = s.find(['^', '$']) {
        let (prop, rest) = s.split_at(pos);
        let marker = rest.chars().next().unwrap();
        let prop = prop.to_string();
        if prop.is_empty() {
            return Err("missing property name before match marker".into());
        }
        let n: usize = rest[1..]
            .parse()
            .map_err(|_| format!("invalid length '{}' in '{}'", &rest[1..], s))?;
        if n == 0 {
            return Err(format!("length must be > 0 in '{}'", s));
        }
        let mode = match marker {
            '^' => MatchMode::Prefix(n),
            '$' => MatchMode::Suffix(n),
            _ => unreachable!(),
        };
        Ok((prop, mode))
    } else {
        Ok((s.to_string(), MatchMode::Exact))
    }
}

pub fn swallow_regex(value: &str, mode: &MatchMode) -> String {
    match mode {
        MatchMode::Exact => format!("^{}$", escape(value)),
        MatchMode::Prefix(n) => {
            let prefix: String = value.chars().take(*n).collect();
            format!("^{}", escape(&prefix))
        }
        MatchMode::Suffix(n) => {
            let chars: Vec<char> = value.chars().collect();
            let start = chars.len().saturating_sub(*n);
            let suffix: String = chars[start..].iter().collect();
            format!("{}$", escape(&suffix))
        }
    }
}

fn deserialize_swallow_criteria<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, Vec<HashMap<String, MatchMode>>>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = HashMap::<String, SwallowCriteriaValue>::deserialize(deserializer)?;
    let mut out = HashMap::new();
    for (k, v) in raw {
        let sets: Vec<Vec<String>> = match v {
            SwallowCriteriaValue::Single(s) => vec![s],
            SwallowCriteriaValue::Multiple(m) => m,
        };
        let mut parsed_sets = Vec::new();
        for set in sets {
            let mut map = HashMap::new();
            for entry in set {
                let (prop, mode) =
                    parse_criterion(&entry).map_err(serde::de::Error::custom)?;
                if map.insert(prop.clone(), mode).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate property '{}' in swallow criteria set",
                        prop
                    )));
                }
            }
            parsed_sets.push(map);
        }
        out.insert(k, parsed_sets);
    }
    Ok(out)
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
    pub window_swallow_criteria: HashMap<String, Vec<HashMap<String, MatchMode>>>,
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
