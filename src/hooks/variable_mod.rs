use regex::Regex;
use rhai::{Array, Dynamic, Module};

use crate::interactive::prompt_and_check_variable;
use crate::project_variables::{StringEntry, StringKind, TemplateSlots, VarInfo};
use crate::template::LiquidObjectResource;

use super::{HookResult, PoisonError};

pub fn create_module(liquid_object: &LiquidObjectResource) -> Module {
    let mut module = Module::new();

    module.set_native_fn("is_set", {
        let liquid_object = liquid_object.clone();
        move |name: &str| -> HookResult<bool> {
            match liquid_object.get_value(name)? {
                NamedValue::NonExistent => Ok(false),
                _ => Ok(true),
            }
        }
    });

    module.set_native_fn("get", {
        let liquid_object = liquid_object.clone();
        move |name: &str| -> HookResult<Dynamic> {
            match liquid_object.get_value(name)? {
                NamedValue::NonExistent => Ok(Dynamic::from(String::from(""))),
                NamedValue::Bool(v) => Ok(Dynamic::from(v)),
                NamedValue::String(v) => Ok(Dynamic::from(v)),
            }
        }
    });

    module.set_native_fn("set", {
        let liquid_object = liquid_object.clone();
        move |name: &str, value: &str| -> HookResult<()> {
            match liquid_object.get_value(name)? {
                NamedValue::NonExistent | NamedValue::String(_) => {
                    liquid_object
                        .lock()
                        .map_err(|_| PoisonError::new_eval_alt_result())?
                        .borrow_mut()
                        .insert(
                            name.to_string(),
                            serde_json::Value::from(value.to_string()),
                        );
                    Ok(())
                }
                _ => Err(format!("Variable {name} not a String").into()),
            }
        }
    });

    module.set_native_fn("set", {
        let liquid_object = liquid_object.clone();
        move |name: &str, value: bool| -> HookResult<()> {
            match liquid_object.get_value(name)? {
                NamedValue::NonExistent | NamedValue::Bool(_) => {
                    liquid_object
                        .lock()
                        .map_err(|_| PoisonError::new_eval_alt_result())?
                        .borrow_mut()
                        .insert(name.to_string(), serde_json::Value::from(value));
                    Ok(())
                }
                _ => Err(format!("Variable {name} not a bool").into()),
            }
        }
    });

    module.set_native_fn("set", {
        let liquid_object = liquid_object.clone();
        move |name: &str, value: Array| -> HookResult<()> {
            match liquid_object.get_value(name)? {
                NamedValue::NonExistent => {
                    let val = rhai_to_liquid_value(Dynamic::from(value))?;
                    liquid_object
                        .lock()
                        .map_err(|_| PoisonError::new_eval_alt_result())?
                        .borrow_mut()
                        .insert(name.to_string(), val);
                    Ok(())
                }
                _ => Err(format!("Variable {name} not an array").into()),
            }
        }
    });

    module.set_native_fn("prompt", {
        move |prompt: &str, default_value: bool| -> HookResult<bool> {
            let value = prompt_and_check_variable(
                &TemplateSlots {
                    prompt: prompt.into(),
                    var_name: "".into(),
                    var_info: VarInfo::Bool {
                        default: Some(default_value),
                    },
                },
                None,
            );

            match value {
                Ok(v) => Ok(v.parse::<bool>().map_err(|_| "Unable to parse into bool")?),
                Err(e) => Err(e.to_string().into()),
            }
        }
    });

    module.set_native_fn("prompt", {
        move |prompt: &str| -> HookResult<String> {
            let value = prompt_and_check_variable(
                &TemplateSlots {
                    prompt: prompt.into(),
                    var_name: "".into(),
                    var_info: VarInfo::String {
                        entry: Box::new(StringEntry {
                            default: None,
                            kind: StringKind::String,
                            regex: None,
                        }),
                    },
                },
                None,
            );

            match value {
                Ok(v) => Ok(v),
                Err(e) => Err(e.to_string().into()),
            }
        }
    });

    module.set_native_fn("prompt", {
        move |prompt: &str, default_value: &str| -> HookResult<String> {
            let value = prompt_and_check_variable(
                &TemplateSlots {
                    prompt: prompt.into(),
                    var_name: "".into(),
                    var_info: VarInfo::String {
                        entry: Box::new(StringEntry {
                            default: Some(default_value.into()),
                            kind: StringKind::String,
                            regex: None,
                        }),
                    },
                },
                None,
            );

            match value {
                Ok(v) => Ok(v),
                Err(e) => Err(e.to_string().into()),
            }
        }
    });

    module.set_native_fn("prompt", {
        move |prompt: &str, default_value: &str, regex: &str| -> HookResult<String> {
            let value = prompt_and_check_variable(
                &TemplateSlots {
                    prompt: prompt.into(),
                    var_name: "".into(),
                    var_info: VarInfo::String {
                        entry: Box::new(StringEntry {
                            default: Some(default_value.into()),
                            kind: StringKind::String,
                            regex: Some(Regex::new(regex).map_err(|_| "Invalid regex")?),
                        }),
                    },
                },
                None,
            );

            match value {
                Ok(v) => Ok(v),
                Err(e) => Err(e.to_string().into()),
            }
        }
    });

    module.set_native_fn("prompt", {
        move |prompt: &str, default_value: &str, choices: rhai::Array| -> HookResult<String> {
            let value = prompt_and_check_variable(
                &TemplateSlots {
                    prompt: prompt.into(),
                    var_name: "".into(),
                    var_info: VarInfo::String {
                        entry: Box::new(StringEntry {
                            default: Some(default_value.into()),
                            kind: StringKind::Choices(
                                choices
                                    .iter()
                                    .map(|d| d.to_owned().into_string().unwrap())
                                    .collect(),
                            ),
                            regex: None,
                        }),
                    },
                },
                None,
            );

            match value {
                Ok(v) => Ok(v),
                Err(e) => Err(e.to_string().into()),
            }
        }
    });

    module
}

enum NamedValue {
    NonExistent,
    Bool(bool),
    String(String),
}

trait GetNamedValue {
    fn get_value(&self, name: &str) -> HookResult<NamedValue>;
}

impl GetNamedValue for LiquidObjectResource {
    fn get_value(&self, name: &str) -> HookResult<NamedValue> {
        let lock = self.lock()
            .map_err(|_| PoisonError::new_eval_alt_result())?;
        let obj = lock.borrow();
        
        if let Some(value) = obj.get(name) {
            // Try to interpret as bool first
            if let Some(b) = value.as_bool() {
                Ok(NamedValue::Bool(b))
            } else if let Some(s) = value.as_str() {
                Ok(NamedValue::String(s.to_string()))
            } else {
                // Try to convert to string
                Ok(NamedValue::String(value.to_string()))
            }
        } else {
            Ok(NamedValue::NonExistent)
        }
    }
}

fn rhai_to_liquid_value(val: Dynamic) -> HookResult<serde_json::Value> {
    if let Some(b) = val.clone().try_cast::<bool>() {
        return Ok(serde_json::Value::from(b));
    }
    
    if let Ok(s) = val.clone().into_string() {
        return Ok(serde_json::Value::from(s));
    }
    
    if let Some(arr) = val.clone().try_cast::<Array>() {
        let items: HookResult<Vec<serde_json::Value>> = arr
            .into_iter()
            .map(rhai_to_liquid_value)
            .collect();
        return items.map(serde_json::Value::from);
    }
    
    Err(format!(
        "expecting type to be string, bool or array but found a '{}' instead",
        val.type_name()
    )
    .into())
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        sync::{Arc, Mutex},
    };

    use super::*;

    #[test]
    fn test_rhai_set() {
        let mut engine = rhai::Engine::new();
        let liquid_object = Arc::new(Mutex::new(RefCell::new(serde_json::Map::new())));

        let module = create_module(&liquid_object);
        engine.register_static_module("variable", module.into());

        engine
            .eval::<()>(
                r#"
            let dependencies = ["some_dep", "other_dep"];

            variable::set("dependencies", dependencies);
        "#,
            )
            .unwrap();

        let ref_cell = liquid_object.lock().unwrap();
        let liquid_object = ref_cell.borrow();

        let deps_value = liquid_object.get("dependencies");
        assert!(deps_value.is_some());
        
        // Check that it's an array with the expected values
        if let Some(val) = deps_value {
            if let Some(arr) = val.as_array() {
                assert_eq!(arr.len(), 2);
                assert_eq!(arr[0].as_str(), Some("some_dep"));
                assert_eq!(arr[1].as_str(), Some("other_dep"));
            } else {
                panic!("Expected array value for dependencies");
            }
        }
    }
}
