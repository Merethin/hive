use std::{path::Path, error::Error, fs::read_to_string};
use serde::Deserialize;
use serde_json::{Value, Map};

use crate::{cache::Cache, loaders::categories::parse_categories};

#[derive(Debug, Deserialize)]
struct TemplateConfig {
    title: String,
    category: String,
    nation: String,
    id: Option<u64>,
    regenerate_after: Option<u64>,
}

#[derive(Debug)]
pub struct TemplateMetadata {
    pub id: String,
    pub title: String,
    pub category: u64,
    pub subcategory: u64,
    pub nation: String,
    pub dispatchid: Option<u64>,
    pub regen: Option<u64>,
}

impl TemplateMetadata {
    pub fn to_obj(&self, cache: &Cache) -> Value {
        let mut value = Map::new();

        let dispatchid = self.dispatchid.or(cache.id(&self.id));

        value.insert("title".into(), Value::String(self.title.clone()));
        value.insert("url".into(), Value::String(match dispatchid {
            None => "https://www.nationstates.net".into(),
            Some(id) => format!("https://www.nationstates.net/page=dispatch/id={}", id),
        }));

        Value::Object(value)
    }

    pub fn make_context(metadata: &Vec<TemplateMetadata>, cache: &Cache) -> Value {
        let mut value = Map::new();

        for template in metadata {
            value.insert(template.id.clone(), template.to_obj(cache));
        }

        Value::Object(value)
    }
}

const INDEX_FILE: &'static str = "index.toml";

pub fn read_template_metadata(data_root: &Path) -> Result<Vec<TemplateMetadata>, Box<dyn Error>> {
    let index_file = read_to_string(data_root.join(INDEX_FILE))?;
    let index_data = toml::from_str::<toml::Table>(&index_file)?;
    let mut templates = Vec::new();

    for (id, value) in index_data.iter() {
        let template_data = TemplateConfig::deserialize(value.clone())?;
        let Some((category, subcategory)) = parse_categories(&template_data.category) else {
            continue;
        };

        templates.push(TemplateMetadata { 
            id: id.clone(), 
            title: template_data.title, 
            category: category, 
            subcategory: subcategory, 
            nation: template_data.nation, 
            dispatchid: template_data.id,
            regen: template_data.regenerate_after,
        });
    }

    Ok(templates)
}