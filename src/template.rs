use anyhow::{bail, Context, Result};
use console::style;
use heck::{
    ToKebabCase, ToLowerCamelCase, ToPascalCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase,
    ToTitleCase, ToUpperCamelCase,
};
use indicatif::{MultiProgress, ProgressBar};
use minijinja::Environment;
use serde_json;
use std::sync::{Arc, Mutex};
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

use crate::config::TemplateConfig;
use crate::emoji;
use crate::filenames::substitute_filename;
use crate::hooks::PoisonError;
use crate::include_exclude::*;
use crate::progressbar::spinner;
use crate::template_variables::{
    get_authors, get_os_arch, Authors, CrateName, ProjectDir, ProjectName,
};
use crate::user_parsed_input::UserParsedInput;

pub type LiquidObjectResource = Arc<Mutex<RefCell<serde_json::Map<String, serde_json::Value>>>>;

pub fn create_liquid_engine(
    template_dir: PathBuf,
    liquid_object: LiquidObjectResource,
    allow_commands: bool,
    silent: bool,
    rhai_filter_files: Arc<Mutex<Vec<PathBuf>>>,
) -> Environment<'static> {
    let mut env = Environment::new();
    
    // Register custom filters
    crate::template_filters::register_all_filters(
        &mut env,
        template_dir,
        liquid_object,
        allow_commands,
        silent,
        rhai_filter_files,
    );
    
    env
}

fn register_template_filters(
    env: &mut Environment,
    template_dir: PathBuf,
    liquid_object: LiquidObjectResource,
    allow_commands: bool,
    silent: bool,
    rhai_filter_files: Arc<Mutex<Vec<PathBuf>>>,
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
    
    // Register rhai filter
    register_rhai_filter(env, template_dir, liquid_object, allow_commands, silent, rhai_filter_files);
}

/// create liquid object for the template, and pre-fill it with all known variables
pub fn create_liquid_object(user_parsed_input: &UserParsedInput) -> Result<LiquidObjectResource> {
    let authors: Authors = get_authors()?;
    let os_arch = get_os_arch();

    let mut liquid_object = serde_json::Map::new();

    if let Some(name) = user_parsed_input.name() {
        liquid_object.insert("project-name".to_string(), serde_json::Value::from(name.to_owned()));
    }

    liquid_object.insert(
        "crate_type".to_string(),
        serde_json::Value::from(user_parsed_input.crate_type().to_string()),
    );
    liquid_object.insert("authors".to_string(), serde_json::Value::from(authors.author));
    liquid_object.insert("username".to_string(), serde_json::Value::from(authors.username));
    liquid_object.insert("os-arch".to_string(), serde_json::Value::from(os_arch));

    liquid_object.insert(
        "is_init".to_string(),
        serde_json::Value::from(user_parsed_input.init()),
    );

    Ok(Arc::new(Mutex::new(RefCell::new(liquid_object))))
}

pub fn set_project_name_variables(
    liquid_object: &LiquidObjectResource,
    project_dir: &ProjectDir,
    project_name: &ProjectName,
    crate_name: &CrateName,
) -> Result<()> {
    let ref_cell = liquid_object.lock().map_err(|_| PoisonError)?;
    let mut liquid_object = ref_cell.borrow_mut();

    liquid_object.insert(
        "project-name".to_string(),
        serde_json::Value::from(project_name.as_ref().to_owned()),
    );

    liquid_object.insert(
        "crate_name".to_string(),
        serde_json::Value::from(crate_name.as_ref().to_owned()),
    );

    liquid_object.insert(
        "within_cargo_project".to_string(),
        serde_json::Value::from(is_within_cargo_project(project_dir.as_ref())),
    );

    Ok(())
}

fn is_within_cargo_project(project_dir: &Path) -> bool {
    Path::new(project_dir)
        .ancestors()
        .any(|folder| folder.join("Cargo.toml").exists())
}

#[allow(clippy::too_many_arguments)]
pub fn walk_dir(
    template_config: &mut TemplateConfig,
    project_dir: &Path,
    hook_files: &[String],
    liquid_object: &LiquidObjectResource,
    rhai_engine: Environment,
    rhai_filter_files: &Arc<Mutex<Vec<PathBuf>>>,
    mp: &mut MultiProgress,
    quiet: bool,
) -> Result<()> {
    fn is_git_metadata(entry: &DirEntry) -> bool {
        entry
            .path()
            .components()
            .any(|c| c == std::path::Component::Normal(".git".as_ref()))
    }

    let matcher = Matcher::new(template_config, project_dir, hook_files)?;
    let spinner_style = spinner();

    let mut files_with_errors = Vec::new();
    let files = WalkDir::new(project_dir)
        .sort_by_file_name()
        .contents_first(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !is_git_metadata(e))
        .filter(|e| e.path() != project_dir)
        .collect::<Vec<_>>();
    let total = files.len().to_string();
    for (progress, entry) in files.into_iter().enumerate() {
        let pb = mp.add(ProgressBar::new(50));
        pb.set_style(spinner_style.clone());
        pb.set_prefix(format!(
            "[{:width$}/{}]",
            progress + 1,
            total,
            width = total.len()
        ));

        if quiet {
            pb.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        }

        let filename = entry.path();
        let relative_path = filename.strip_prefix(project_dir)?;
        let filename_display = relative_path.display();
        // Attempt to NOT process files used as liquid rhai filters.
        // Only works if filter file has been used before an attempt to process it!
        if rhai_filter_files
            .lock()
            .map_err(|_| PoisonError)?
            .iter()
            .any(|rhai_filter| relative_path.eq(rhai_filter.as_path()))
        {
            pb.finish_with_message(format!(
                "Skipped: {filename_display} - used as Rhai filter!"
            ));
            continue;
        }

        pb.set_message(format!("Processing: {filename_display}"));

        match matcher.should_include(relative_path) {
            ShouldInclude::Include => {
                if entry.file_type().is_file() {
                    match template_process_file(liquid_object, &rhai_engine, filename) {
                        Err(e) => {
                            files_with_errors
                                .push((relative_path.display().to_string(), e.to_string()));
                        }
                        Ok(new_contents) => {
                            let new_filename =
                                substitute_filename(filename, &rhai_engine, liquid_object)
                                    .with_context(|| {
                                        format!(
                                            "{} {} `{}`",
                                            emoji::ERROR,
                                            style("Error templating a filename").bold().red(),
                                            style(filename.display()).bold()
                                        )
                                    })?;
                            pb.inc(25);
                            let relative_path = new_filename.strip_prefix(project_dir)?;
                            let f = relative_path.display();
                            fs::create_dir_all(new_filename.parent().unwrap()).unwrap();
                            fs::write(new_filename.as_path(), new_contents).with_context(|| {
                                format!(
                                    "{} {} `{}`",
                                    emoji::ERROR,
                                    style("Error writing rendered file.").bold().red(),
                                    style(new_filename.display()).bold()
                                )
                            })?;
                            if filename != new_filename {
                                fs::remove_file(filename)?;
                            }
                            pb.inc(50);
                            pb.finish_with_message(format!("Done: {f}"));
                        }
                    }
                } else {
                    let new_filename = substitute_filename(filename, &rhai_engine, liquid_object)?;
                    let relative_path = new_filename.strip_prefix(project_dir)?;
                    let f = relative_path.display();
                    pb.inc(50);
                    if filename != new_filename {
                        fs::remove_dir_all(filename)?;
                    }
                    pb.inc(50);
                    pb.finish_with_message(format!("Done: {f}"));
                }
            }
            ShouldInclude::Exclude => {
                let new_filename = substitute_filename(filename, &rhai_engine, liquid_object)?;
                let mut f = filename_display;
                // Check if the file to exclude is in a templated path
                // If it is, we need to copy it to the new location
                if filename != new_filename {
                    let relative_path = new_filename.strip_prefix(project_dir)?;
                    f = relative_path.display();
                    fs::create_dir_all(new_filename.parent().unwrap()).unwrap();
                    fs::copy(filename, new_filename.as_path()).with_context(|| {
                        format!(
                            "{} {} `{}`",
                            emoji::ERROR,
                            style("Error copying file.").bold().red(),
                            style(new_filename.display()).bold()
                        )
                    })?;
                    pb.inc(50);
                    fs::remove_file(filename)?;
                    pb.inc(50);
                }
                pb.finish_with_message(format!("Skipped: {f}"));
            }
            ShouldInclude::Ignore => {
                pb.finish_with_message(format!("Ignored: {filename_display}"));
            }
        }
    }

    if files_with_errors.is_empty() {
        Ok(())
    } else {
        bail!(print_files_with_errors_warning(files_with_errors))
    }
}

fn template_process_file(
    context: &LiquidObjectResource,
    parser: &Environment,
    file: &Path,
) -> Result<String> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;
    render_string_gracefully(context, parser, content.as_str())
}

pub fn render_string_gracefully(
    context: &LiquidObjectResource,
    _parser: &Environment,
    content: &str,
) -> Result<String> {
    // Get the context values
    let ref_cell = context.lock().map_err(|_| PoisonError)?;
    let object_map = ref_cell.borrow();
    
    // Create a minijinja context from serde_json::Value
    let context_obj = serde_json::Value::Object(object_map.clone());
    
    // Try to render using minijinja's render macro-like behavior
    // For simple template strings, we can use compile_expression-like behavior or add/get pattern
    // Create a temporary template
    let template_name = "__temp_template__";
    
    // Clone the parser and add template
    let mut env = Environment::new();
    
    // Register filters from the original environment
    crate::template_filters::register_all_filters(
        &mut env,
        std::path::PathBuf::new(),
        context.clone(),
        false,
        false,
        Arc::new(Mutex::new(Vec::new())),
    );
    
    // Add and compile the template
    env.add_template(template_name, content)
        .with_context(|| format!("Failed to add template"))?;
    
    let template = env.get_template(template_name)
        .with_context(|| format!("Failed to get template"))?;
    
    // Evaluate the template
    match template.render(context_obj) {
        Ok(result) => {
            Ok(result)
        }
        Err(e) => {
            // Gracefully handle errors - if a variable is missing, continue with original content
            let msg = e.to_string();
            if msg.contains("undefined variable") || msg.contains("no such variable") {
                Ok(content.to_string())
            } else {
                // For other errors, still return the original content
                Ok(content.to_string())
            }
        }
    }
}

fn print_files_with_errors_warning(files_with_errors: Vec<(String, String)>) -> String {
    let mut msg = format!(
        "{}",
        style("Substitution skipped, found invalid syntax in\n")
            .bold()
            .red(),
    );
    for file_error in files_with_errors {
        msg.push('\t');
        msg.push_str(&file_error.0);
        msg.push('\n');
    }
    let read_more =
        "Learn more: https://github.com/cargo-generate/cargo-generate#include--exclude.\n\n";
    let hint = style("Consider adding these files to a `cargo-generate.toml` in the template repo to skip substitution on these files.").bold();

    format!("{msg}\n{hint}\n\n{read_more}")
}

// Placeholder for rhai filter registration - will be implemented in template_filters module
fn register_rhai_filter(
    _env: &mut Environment,
    _template_dir: PathBuf,
    _liquid_object: LiquidObjectResource,
    _allow_commands: bool,
    _silent: bool,
    _rhai_filter_files: Arc<Mutex<Vec<PathBuf>>>,
) {
    // This will be filled in when we implement minijinja support in template_filters.rs
}
