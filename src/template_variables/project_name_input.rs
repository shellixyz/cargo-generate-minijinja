use std::fmt::Display;

use anyhow::anyhow;
use console::style;

use crate::{
    emoji, interactive, template::LiquidObjectResource, user_parsed_input::UserParsedInput,
};
use log::warn;

#[derive(Debug)]
pub struct ProjectNameInput(pub(crate) String);

impl TryFrom<(&LiquidObjectResource, &UserParsedInput)> for ProjectNameInput {
    type Error = anyhow::Error;

    fn try_from(
        (liquid_object, user_parsed_input): (&LiquidObjectResource, &UserParsedInput),
    ) -> Result<Self, Self::Error> {
        let name_str = {
            let guard = liquid_object.lock().unwrap();
            let borrowed_obj = guard.borrow();
            
            // Try underscore version first (set by init hooks) then fall back to hyphenated version (from CLI)
            borrowed_obj
                .get("project_name")
                .or_else(|| borrowed_obj.get("project-name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        };
        
        let name = name_str
            .as_deref()
            .map(|v| {
                if let Some(n) = user_parsed_input.name() {
                    if n != v {
                        warn!(
                            "{} `{}` {} `{}`{}",
                            style("Project name changed by template, from").bold(),
                            style(n).bold().yellow(),
                            style("to").bold(),
                            style(v).bold().green(),
                            style("...").bold()
                        );
                    }
                }
                v.to_string()
            })
            .or_else(|| user_parsed_input.name().map(String::from));

        match name {
            Some(name) => Ok(Self(name)),
            None => {
                match std::env::var("CARGO_GENERATE_VALUE_PROJECT_NAME") {
                    Ok(name) => Ok(Self(name)),
                    Err(_) if !user_parsed_input.silent() => Ok(Self(interactive::name()?)),
                    Err(_) => Err(anyhow!(
                        "{} {} {}",
                        emoji::ERROR,
                        style("Project Name Error:").bold().red(),
                        style("Option `--silent` provided, but project name was not set. Please use `--name`.")
                            .bold()
                            .red(),
                    )),
                }
            }
        }
    }
}

impl AsRef<str> for ProjectNameInput {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Display for ProjectNameInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
