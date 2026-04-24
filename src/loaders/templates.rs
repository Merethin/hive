use std::{path::Path, error::Error, fs::{read_dir, read_to_string}};
use minijinja::Environment;

use crate::loaders::{metadata::TemplateMetadata, extension, file_prefix};

pub fn load_supporting_templates(
    env: &mut Environment<'_>, data_root: &Path, directory: &str
) -> Result<(), Box<dyn Error>> {
    let template_dir = data_root.join(directory);

    if template_dir.is_dir() {
        for entry in read_dir(template_dir)?.flatten() {
            let path = entry.path();
            
            if extension(&path) == Some("tmpl") && let Some(prefix) = file_prefix(&path) {
                let contents = read_to_string(&path)?;
                env.add_template_owned(format!("{}/{}", directory, prefix), contents)?;
            }
        }
    } else {
        eprintln!("Template directory '{}' does not exist or is not a directory!", directory);
    }

    Ok(())
}

const TEMPLATE_DIR: &'static str = "templates";

pub fn load_main_templates(
    env: &mut Environment<'_>, data_root: &Path, templates: &Vec<TemplateMetadata>
) -> Result<(), Box<dyn Error>> {
    let template_root = data_root.join(TEMPLATE_DIR);

    for template in templates {
        let mut path = template_root.join(&template.id);
        path.add_extension("tmpl");

        let contents = read_to_string(&path)?;
        env.add_template_owned(template.id.clone(), contents)?;
    }

    Ok(())
}