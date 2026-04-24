use std::{collections::{HashMap, hash_map::Keys}, error::Error, fs::File, path::Path};

pub struct Credentials {
    inner: HashMap<String, String>,
}

impl Credentials {
    pub fn empty() -> Self {
        Self {
            inner: HashMap::new()
        }
    }

    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        Ok(Self {
            inner: serde_json::from_reader(f)?
        })
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        serde_json::to_writer_pretty(f, &self.inner)?;
        Ok(())
    }

    pub fn get(&self, nation: &str) -> Option<&String> {
        self.inner.get(nation)
    }

    pub fn set(&mut self, nation: String, token: String) {
        self.inner.insert(nation, token);
    }

    pub fn remove(&mut self, nation: &str) -> bool {
        self.inner.remove(nation).is_some()
    }

    pub fn keys(&self) -> Keys<'_, String, String> {
        self.inner.keys()
    }
}