#![allow(clippy::box_default)]

use anyhow::Result;
use console::style;
use heck::{
    ToKebabCase, ToLowerCamelCase, ToPascalCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase,
    ToTitleCase, ToUpperCamelCase,
};
use log::warn;
use minijinja::Environment;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    hooks::{create_rhai_engine, PoisonError, RhaiHooksContext},
    template::LiquidObjectResource,
};

// This file is now mainly a placeholder since minijinja filters are registered
// directly in template.rs using add_filter. The case conversion functions below
// are used by those filters.

/// Helper to register all template filters with a minijinja environment
pub fn register_all_filters(
    env: &mut Environment,
    _template_dir: PathBuf,
    _liquid_object: LiquidObjectResource,
    _allow_commands: bool,
    _silent: bool,
    _rhai_filter_files: Arc<Mutex<Vec<PathBuf>>>,
) {
    // Register case conversion filters
    env.add_filter("kebab_case", |s: String| -> String { s.to_kebab_case() });
    env.add_filter("lower_camel_case", |s: String| -> String { s.to_lower_camel_case() });
    env.add_filter("pascal_case", |s: String| -> String { s.to_pascal_case() });
    env.add_filter("shouty_kebab_case", |s: String| -> String { s.to_shouty_kebab_case() });
    env.add_filter("shouty_snake_case", |s: String| -> String { s.to_shouty_snake_case() });
    env.add_filter("snake_case", |s: String| -> String { s.to_snake_case() });
    env.add_filter("title_case", |s: String| -> String { s.to_title_case() });
    env.add_filter("upper_camel_case", |s: String| -> String { s.to_upper_camel_case() });
}

fn rhai_filter_impl(
    script_path: &str,
    template_dir: PathBuf,
    liquid_object: LiquidObjectResource,
    allow_commands: bool,
    silent: bool,
    rhai_filter_files: Arc<Mutex<Vec<PathBuf>>>,
) -> Result<String> {
    let file_path = PathBuf::from(script_path);
    
    if !file_path.exists() {
        warn!(
            "{} {} {}",
            style("Filter script").bold().yellow(),
            style(file_path.display()).bold().red(),
            style("not found").bold().yellow(),
        );
        anyhow::bail!("Filter script {} not found", file_path.display());
    }
    
    rhai_filter_files
        .lock()
        .map_err(|_| anyhow::anyhow!(PoisonError.to_string()))?
        .push(file_path.clone());

    let context = RhaiHooksContext {
        liquid_object,
        allow_commands,
        silent,
        working_directory: template_dir.clone(),
        destination_directory: template_dir,
    };

    let engine = create_rhai_engine(&context);
    match engine.eval_file::<String>(file_path.clone()) {
        Ok(r) => Ok(r),
        Err(err) => {
            warn!(
                "{} {} {} {}",
                style("Filter script").bold().yellow(),
                style(file_path.display()).bold().red(),
                style("contained error").bold().yellow(),
                style(err.to_string()).bold().red(),
            );
            anyhow::bail!("Rhai filter error: {}", err)
        }
    }
}
