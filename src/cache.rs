use std::{collections::{HashMap, hash_map::Entry}, error::Error, fs::File, path::Path};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheData {
    pub id: u64,
    pub hash: String,
    pub created_at: u64,
}

impl CacheData {
    pub fn new(
        id: u64,
        hash: String,
        created_at: u64,
    ) -> Self {
        Self {
            id, hash, created_at
        }
    }
}

#[derive(Debug, Clone)]
pub struct Cache {
    inner: HashMap<String, CacheData>,
}

impl Cache {
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

    pub fn set(&mut self, dispatch: String, data: CacheData) {
        self.inner.insert(dispatch, data);
    }

    pub fn update(&mut self, dispatch: String, data: CacheData) {
        match self.inner.entry(dispatch) {
            Entry::Vacant(v) => {
                v.insert(data);
            },
            Entry::Occupied(mut o) => {
                let entry = o.get_mut();
                entry.id = data.id;
                entry.hash = data.hash;
            }
        }
    }

    pub fn id(&self, dispatch: &str) -> Option<u64> {
        self.inner.get(dispatch).map(|v| v.id)
    }

    pub fn hash(&self, dispatch: &str) -> Option<&str> {
        self.inner.get(dispatch).map(|v| v.hash.as_str())
    }

    pub fn created_at(&self, dispatch: &str) -> Option<u64> {
        self.inner.get(dispatch).map(|v| v.created_at)
    }
}