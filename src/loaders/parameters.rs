use std::{path::Path, error::Error, fs::{read_dir, read_to_string}};
use serde_json::{Map, Value};

use crate::loaders::{extension, file_prefix};

pub fn load_parameters(directory: &Path) -> Result<Value, Box<dyn Error>> {
    let mut value = Map::new();

    if directory.is_dir() {
        for entry in read_dir(directory)?.flatten() {
            let path = entry.path();

            let ext = extension(&path);
            if ext != Some("json") && ext != Some("toml") {
                continue;
            }

            let contents = read_to_string(entry.path())?;

            let obj = match ext {
                Some("json") => serde_json::from_str::<Value>(&contents)?,
                Some("toml") => toml::from_str::<Value>(&contents)?,
                _ => unreachable!(),
            };

            if let Some(prefix) = file_prefix(&path) {
                value.insert(prefix, obj);
            }
        }
    } else {
        eprintln!("Parameter directory '{:?}' does not exist or is not a directory!", directory);
    }

    Ok(Value::Object(value))
}