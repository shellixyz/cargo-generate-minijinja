#![allow(clippy::box_default)]

use heck::{
    ToKebabCase, ToLowerCamelCase, ToPascalCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase,
    ToTitleCase, ToUpperCamelCase,
};
use minijinja::Environment;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::template::LiquidObjectResource;
use log::warn;

// This file is now mainly a placeholder since minijinja filters are registered
// directly in template.rs using add_filter. The case conversion functions below
// are used by those filters.

/// Helper to register all template filters with a minijinja environment
pub fn register_all_filters(
    env: &mut Environment,
    template_dir: PathBuf,
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
    
    // Register date filter - simple implementation extracting year from date string
    env.add_filter("date", date_filter);
    
    // Register rhai filter - execute rhai scripts
    let template_dir_clone = template_dir.clone();
    env.add_filter("rhai", move |filename: String| -> String { 
        rhai_filter(&filename, &template_dir_clone)
    });
}

fn date_filter(date_str: String, format_str: String) -> String {
    // Simple date filter that extracts portions of the date string
    // For now, just handle the common case of extracting the year (%Y)
    match format_str.as_str() {
        "%Y" => {
            // Extract year (first 4 digits)
            date_str.chars().take(4).collect()
        },
        "%m" => {
            // Extract month (characters 5-6 in YYYY-MM-DD format)
            if date_str.len() >= 7 {
                date_str.chars().skip(5).take(2).collect()
            } else {
                date_str
            }
        },
        "%d" => {
            // Extract day (characters 8-9 in YYYY-MM-DD format)
            if date_str.len() >= 10 {
                date_str.chars().skip(8).take(2).collect()
            } else {
                date_str
            }
        },
        _ => {
            // For unsupported formats, return the original string
            date_str
        }
    }
}

fn rhai_filter(filename: &str, template_dir: &PathBuf) -> String {
    use std::fs;
    
    // Construct the full path to the rhai script file
    let script_path = template_dir.join(filename);
    
    // Check if the file exists
    if !script_path.exists() {
        warn!("Filter script {} not found", filename);
        // Return the filename unchanged so the filter expression remains visible
        return filename.to_string();
    }
    
    // Read the script file
    match fs::read_to_string(&script_path) {
        Ok(script_content) => {
            // Execute the rhai script and capture the result
            match execute_rhai_script(&script_content) {
                Ok(result) => result,
                Err(e) => {
                    warn!("Failed to execute rhai script {}: {}", filename, e);
                    filename.to_string()
                }
            }
        }
        Err(e) => {
            warn!("Failed to read rhai script {}: {}", filename, e);
            filename.to_string()
        }
    }
}

fn execute_rhai_script(script_content: &str) -> Result<String, Box<dyn std::error::Error>> {
    use rhai::Engine;
    
    let mut engine = Engine::new();
    let result: rhai::Dynamic = engine.eval(script_content)?;
    Ok(result.to_string())
}
