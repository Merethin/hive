mod loaders;
mod filters;
mod credentials;
mod api;
mod cache;

use clap::{Parser, Subcommand};
use itertools::Itertools;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tokio::runtime::Runtime;

use std::{path::Path, process::exit, time::{Duration, SystemTime, UNIX_EPOCH}};
use minijinja::Environment;
use rpassword::prompt_password;
use serde::Serialize;

use filters::register_filters;
use loaders::{TemplateMetadata, load_main_templates, load_parameters, load_supporting_templates, read_template_metadata};
use credentials::Credentials;

use crate::{api::Session, cache::{Cache, CacheData}};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short('c'), long)]
    credential_file_path: Option<String>,

    #[arg(short('C'), long)]
    cache_file_path: Option<String>,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    AddCredential {
        #[arg(short, long)]
        user_agent: String,

        nation: String,
    },
    RemoveCredential {
        nation: String,
    },
    ListCredentials,
    PurgeCache,
    Sync {
        #[arg(short, long)]
        user_agent: String,

        data_path: String,

        #[arg(short, long)]
        generated_data_path: Option<String>,
    },
}

#[derive(Serialize)]
struct TemplateContext {
    parameters: Value,
    generated: Value,
    templates: Value,
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn format_user_agent(user_agent: String) -> String {
    format!("hive/{} by Merethin, used by {}", VERSION, user_agent)
}

fn main() {
    let args = Args::parse();

    let credential_file_path = args.credential_file_path.unwrap_or("credentials.json".into());
    let cache_file_path = args.cache_file_path.unwrap_or("cache.json".into());

    match args.cmd {
        Commands::Sync { 
            user_agent, data_path, generated_data_path 
        } => sync(format_user_agent(user_agent), data_path, generated_data_path, credential_file_path, cache_file_path),

        Commands::AddCredential { 
            user_agent, nation 
        } => add_credential(format_user_agent(user_agent), credential_file_path, nation),

        Commands::RemoveCredential { nation } => remove_credential(credential_file_path, nation),
        Commands::ListCredentials => list_credentials(credential_file_path),
        Commands::PurgeCache => purge(cache_file_path),
    }
}

fn sync(
    user_agent: String,
    data_path: String, 
    generated_data_path: Option<String>, 
    credential_file_path: String,
    cache_file_path: String,
) {
    let mut env = Environment::new();
    register_filters(&mut env);

    let credentials = Credentials::load(Path::new(&credential_file_path)).unwrap_or_else(|_| {
        eprintln!("Error: no valid credentials file exists, cannot load!");
        exit(1);
    });

    let cache = Cache::load(Path::new(&cache_file_path)).unwrap_or(Cache::empty());

    let data_root = Path::new(&data_path);
    if !data_root.is_dir() {
        eprintln!("Error: '{}' is not a valid directory!", data_path);
        exit(1);
    }

    load_supporting_templates(&mut env, data_root, "layouts").unwrap_or_else(|err| {
        eprintln!("Warning: failed to load layouts from data folder ({err}). Skipping.");
    });

    load_supporting_templates(&mut env, data_root, "macros").unwrap_or_else(|err| {
        eprintln!("Warning: failed to load macros from data folder ({err}). Skipping.");
    });

    let metadata = read_template_metadata(data_root).unwrap_or_else(|err| {
        eprintln!("Error: failed to load template metadata ({err})!");
        exit(1);
    });

    load_main_templates(&mut env, data_root, &metadata).unwrap_or_else(|err| {
        eprintln!("Error: failed to load main templates ({err})!");
        exit(1);
    });

    let parameters = load_parameters(&data_root.join("parameters")).unwrap_or_else(|_| { Value::Object(Map::new()) });

    let generated = if let Some(generated_path) = generated_data_path {
        load_parameters(Path::new(&generated_path)).unwrap_or_else(|_| { Value::Object(Map::new()) })
    } else {
        Value::Object(Map::new())
    };

    let context = TemplateContext {
        parameters,
        generated,
        templates: TemplateMetadata::make_context(&metadata, &cache)
    };

    let rt = Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(async move {
        do_update(env, context, metadata, credentials, cache, user_agent, cache_file_path).await
    });
}

const SECONDS_PER_DAY: u64 = 86400;

fn should_regenerate(metadata: &TemplateMetadata, cache: &Cache) -> bool {
    if let Some(regen) = metadata.regen && let Some(last_update) = cache.created_at(&metadata.id) {
        let time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
        let delay = (time - last_update) / SECONDS_PER_DAY;
        delay >= regen
    } else {
        false
    }
}

async fn do_update(
    env: Environment<'_>,
    context: TemplateContext,
    mut metadata: Vec<TemplateMetadata>,
    credentials: Credentials,
    mut cache: Cache,
    user_agent: String,
    cache_file_path: String,
) {
    let mut session = Session::new(user_agent, Some(credentials)).unwrap();

    for template in &mut metadata {
        let content = match env.get_template(&template.id).and_then(|t| t.render(&context)) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("Failed to render template {}: {}", template.id, err);
                continue;
            }
        };

        let regen = should_regenerate(&template, &cache);

        let hash = format!("{:x}", Sha256::digest(&content));
        if !regen && Some(hash.as_str()) == cache.hash(&template.id) {
            continue; // Dispatch content has not changed
        }

        let dispatchid = template.dispatchid.or_else(|| {
            if !regen { cache.id(&template.id) } else { None }
        });

        if let Some(dispatchid) = dispatchid {
            if let Ok(Some(dispatchid)) = session.edit_dispatch(
                &template.nation, &template.title, template.category, template.subcategory, dispatchid, &content
            ).await {
                let time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
                cache.update(template.id.clone(), CacheData::new(dispatchid, hash, time));
                println!("Edited dispatch {} - https://www.nationstates.net/page=dispatch/id={}", template.id, dispatchid);
            } else {
                eprintln!("Failed to edit dispatch {}!", template.id);
            }
        } else {
            if let Ok(Some(dispatchid)) = session.create_dispatch(
                &template.nation, &template.title, template.category, template.subcategory, &content
            ).await {
                let time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
                cache.set(template.id.clone(), CacheData::new(dispatchid, hash, time));
                println!("Created dispatch {} - https://www.nationstates.net/page=dispatch/id={}", template.id, dispatchid);
            } else {
                eprintln!("Failed to create dispatch {}!", template.id);
            }
        }

        tokio::time::sleep(Duration::from_millis(1500)).await;
    }

    cache.save(Path::new(&cache_file_path)).unwrap_or_else(|err| {
        eprintln!("Failed to update cache: {err}");
    })
}

fn add_credential(user_agent: String, credential_file_path: String, nation: String) {
    let path = Path::new(&credential_file_path);
    let mut credentials = Credentials::load(path).unwrap_or_else(|_| {
        eprintln!("No valid credentials file exists, creating new one.");
        Credentials::empty()
    });

    let Ok(password) = prompt_password("Enter nation password: ") else {
        eprintln!("Failed to read password!");
        return;
    };

    let rt = Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(async move {
        let mut session = Session::new(user_agent, None).unwrap();
        let Some(autologin) = session.get_autologin_token(&nation, password).await.ok().flatten() else {
            eprintln!("Failed to obtain autologin token from NationStates!");
            return;
        };

        credentials.set(nation, autologin);
        credentials.save(path).unwrap_or_else(|err| {
            eprintln!("Failed to update credentials: {err}");
        });
    });
    
}

fn remove_credential(credential_file_path: String, nation: String) {
    let path = Path::new(&credential_file_path);
    let mut credentials = Credentials::load(path).unwrap_or_else(|_| {
        eprintln!("No valid credentials file exists, cannot load!");
        exit(1);
    });

    let success = credentials.remove(&nation);
    if !success {
        eprintln!("No credentials entry for nation: {}!", nation);
    } else {
        credentials.save(path).unwrap_or_else(|err| {
            eprintln!("Failed to update credentials: {err}");
        });
    }
}

fn list_credentials(credential_file_path: String) {
    let credentials = Credentials::load(Path::new(&credential_file_path)).unwrap_or_else(|_| {
        eprintln!("No valid credentials file exists, cannot load!");
        exit(1);
    });

    println!("Credentials are set for the following nations: {}", credentials.keys().join(", "));
}

fn purge(cache_file_path: String) {
    Cache::empty().save(Path::new(&cache_file_path)).unwrap_or_else(|_| {
        eprintln!("Failed to empty cache!");
        exit(1);
    });

    println!("Cache has been purged successfully.");
}