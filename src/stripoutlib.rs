// This code is nearly a 1:1 mapping of https://github.com/kynan/nbstripout/blob/master/nbstripout/_utils.py
use log;
use regex::Regex;
use serde_json::json;
use std::borrow::Borrow;

type JSONMap = serde_json::Map<String, serde_json::Value>;

// Given a key 'a.b.c' find and remove it
// e.g. "a.b": {"c": "d"} OR "a": {"b": {"c": "d"}}
fn pop_recursive(d: &mut serde_json::Value, key: &str) {
    let obj = match d.as_object_mut() {
        Some(x) => x,
        None => return,
    };
    if obj.contains_key(key) {
        obj.remove_entry(key);
        return;
    }
    if !key.contains(".") {
        return;
    }
    let mut splitter = key.splitn(2, ".");
    let key_head = splitter.next().unwrap();
    let key_tail = splitter.next().unwrap();
    if obj.contains_key(key_head) {
        pop_recursive(obj.get_mut(key_head).unwrap(), &key_tail)
    }
}

/// Should we keep the output of a given cell?
///
/// If the regex is specified, it will be matched against the cell's
/// "text/plain" or "text" output; if a match is found, the cell's output will
/// be discarded unconditionally. If no match is found, the cell output will
/// only be kept if `keep_output` is in the cell metadata or tags, or if
/// the default is to keep it.
///
/// * `cell`: Contents of a cell
/// * `default`: Whether to keep cell output or not by default
/// * `strip_regex`: Regex to use to determine whether output should be stripped
fn determine_keep_output(cell: &JSONMap, default: bool, strip_regex: Option<&Regex>) -> Result<Vec<bool>, String> {

    // Generate a vector with the same length as cell["outputs"],
    // filled with the given value
    let make_filled_output = |value: bool| -> Result<Vec<bool>, String> {
        cell.get("outputs")
            .and_then(|outputs| outputs.as_array())
            .map(|arr| arr.iter().map(|_| value).collect())
            .ok_or("Could not determine number of outputs.".to_string())
    };

    // If there's a metadata key, retrieve the JSON object. Otherwise exit out, and follow
    // the default clear behavior for all cell outputs
    let metadata = match cell.get("metadata").and_then(|value| value.as_object()) {
        Some(obj) => obj,
        None => return make_filled_output(default)
    };

    let has_keep_output_metadata = metadata.contains_key("keep_output");
    let keep_output_metadata =
        has_keep_output_metadata && metadata["keep_output"].as_bool().unwrap_or(false);

    let has_keep_output_tag = metadata
        .get("tags")
        .unwrap_or(&json!([]))
        .as_array()
        .unwrap_or(&vec![])
        .contains(&serde_json::Value::String("keep_output".to_string()));

    if has_keep_output_metadata && has_keep_output_tag && !keep_output_metadata {
        return Err(String::from(
            "cell metadata contradicts tags: `keep_output` is false, but `keep_output` in tags",
        ));
    }

    match strip_regex {
        Some(reg) => {
            let mut result = Vec::new();
            let outputs = cell
                .get("outputs")
                .and_then(|value| value.as_array())
                .ok_or("Could not get cell outputs.")?;

            for output in outputs {
                let obj = output.as_object().ok_or("Cell output is not a JSON object; notebook is malformed.")?;
                if output_matches_regex(obj, reg).unwrap_or(false) {
                    // If there's a regex match, that takes precedence over any other
                    // options
                    result.push(false);
                } else if has_keep_output_tag || has_keep_output_metadata {
                    result.push(true);
                } else {
                    result.push(default);
                }
            }
            Ok(result)
        },
        None => {
            // Regex not provided; if metadata or tags request we keep it, then keep it; otherwise
            // revert to `default` behavior
            if has_keep_output_metadata || has_keep_output_tag {
                make_filled_output(true)
            } else {
                make_filled_output(default)
            }
        }
    }
}

/// If a regex is specified, return true if it matches the output and false otherwise.
///
/// See https://nbformat.readthedocs.io/en/latest/format_description.html for the
/// notebook file spec.
///
/// * `output`: Cell output map with text content to be matched against the regex
/// * `strip_regex`: Regex to match the output against
fn output_matches_regex(
    output: &JSONMap,
    strip_regex: &Regex
) -> Result<bool, String> {
    let output_type = output
        .get("output_type")
        .ok_or("Cell output does not contain an output type; notebook data is malformed.")?
        .as_str()
        .ok_or("Cell output type is not a string; notebook data is malformed.")?;

    let data = match output_type {
        "stream" => {
            output
                .get("text")
                .ok_or("Cell output of type 'stream' does not have 'text' key; notebook is malformed.")?
        },
        "display_data" | "execute_result" => {
            output
                .get("data")
                .ok_or(format!("Cell output of type '{}' does not have 'data' key; notebook is malformed.", output_type))?
                .get("text/plain")
                .ok_or(format!("Cell output of type '{}' does not have 'text/plain' key in its 'data' value; notebook is malformed.", output_type))?
        },
        "error" => {
            return Ok(false)
        }
        _ => return Err("Malformed cell output type.".to_string())
    };

    let joined = data
        .as_array()
        .ok_or("Could not get contents of a cell output.")?
        .iter()
        .map(|line| line.as_str().unwrap_or("").to_string())
        .collect::<Vec<String>>()
        .join("");

    Ok(strip_regex.is_match(&joined))
}

// TODO: add custom errors instead of returning a string
#[cfg_attr(not(feature = "extension-module"), allow(unused))]
pub fn strip_output(
    nb: &mut serde_json::Value,
    keep_output: bool,
    keep_count: bool,
    extra_keys: &Vec<String>,
    drop_empty_cells: bool,
    strip_regex: Option<&str>,
) -> Result<bool, String> {
    log::debug!(
        "keep-output: {}, keep-count: {}, extra-keys: {:?}, drop-empty-cells: {}, strip-regex: {:?}",
        keep_output,
        keep_count,
        extra_keys,
        drop_empty_cells,
        strip_regex
    );
    let mut metadata_keys = Vec::<String>::new();
    let mut cell_keys = Vec::<String>::new();

    let empty_json: serde_json::Value = serde_json::json!({});
    let notebook_metadata = nb
        .get("metadata")
        .unwrap_or(&empty_json)
        .as_object()
        .expect("metadata must be an object");

    let keep_output = keep_output
        || notebook_metadata
            .get("keep_output")
            .unwrap_or(&empty_json)
            .as_bool()
            .unwrap_or(false);

    // First split the extra keys into metadata and cell and error if anything else is passed
    for key in extra_keys {
        if !key.contains(".") {
            return Err(format!(
                "extra key '{}' does not contain a . - must be of the form cell.foo or metadata.bar. Exiting...", key
            ));
        }
        let mut splitter = key.splitn(2, ".");
        let namespace = splitter.next().unwrap();
        let subkey = splitter.next().unwrap();
        if namespace == "metadata" {
            metadata_keys.push(subkey.to_string());
        } else if namespace == "cell" {
            cell_keys.push(subkey.to_string());
        } else {
            return Err(format!(
                "extra key '{}' must be of the form cell.foo or metadata.bar. Exiting...",
                key
            ));
        }
    }

    // Remove all keys from metadata
    let metadata_option: Option<&mut serde_json::Value> = nb.get_mut("metadata");
    if metadata_option.is_some() && !metadata_keys.is_empty() {
        let metadata: &mut serde_json::Value = metadata_option.unwrap();
        for field in metadata_keys {
            pop_recursive(metadata, &field);
        }
    }

    // Now process each cell
    let cells_option: Option<&mut serde_json::Value> = nb.get_mut("cells");
    if let Some(cells) = cells_option.and_then(|c| c.as_array_mut()) {
        // Remove cells that the user wants to drop (e.g. empty cells)
        if drop_empty_cells {
            cells.retain(|cell| {
                // Source is an array of lines
                let source: &serde_json::Value =
                    cell.as_object().expect("Cell must be an object")["source"].borrow();
                if source.is_array() {
                    // If any cell has a line that is not just whitespace, retain it
                    return source
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|line| line.as_str().unwrap_or("").trim().len() > 0);
                } else if source.is_string() {
                    return source.as_str().unwrap_or("").trim().len() > 0;
                } else {
                    panic!("Source must be an string or array: {:?}", source);
                }
            });
        }

        let strip_regex_obj = strip_regex
            .map(|s| Regex::new(s).map_err(|_| format!("'{}' is not a valid regex.", s)))
            .transpose()?;

        // Clean up each cell as required
        for cell_object in cells {
            if !cell_object.is_object() {
                log::debug!("Skipping non-object cell");
                continue;
            }
            let cell = cell_object.as_object_mut().expect("Cell must be an object");

            if cell.contains_key("outputs") {
                // Must come before `let outputs = ...` to avoid borrowing an immutable reference
                // and a mutable reference to `cell` simultaneously
                let keep = determine_keep_output(cell, keep_output, strip_regex_obj.as_ref())?;

                let outputs = cell["outputs"]
                    .as_array_mut()
                    .expect("Outputs must be an array");

                // Default behavior (max_size == 0) strips all outputs.
                if keep.is_empty() {
                    outputs.clear();
                } else {
                    let mut keep_iter = keep.iter();
                    outputs.retain(|_| *keep_iter.next().unwrap());
                }

                // Strip the counts from the outputs that were kept if not keep_count.
                if !keep_count {
                    for output in outputs {
                        let obj = output.as_object_mut().expect("Output should be an object");
                        obj.remove("execution_count");
                    }
                }
            }

            // Remove the prompt_number/execution_count, unless directed otherwise
            if cell.contains_key("prompt_number") && !keep_count {
                cell["prompt_number"] = json!(null);
            }
            if cell.contains_key("execution_count") && !keep_count {
                cell["execution_count"] = json!(null);
            }

            // Always remove some metadata
            for field in &cell_keys {
                pop_recursive(cell_object, &field);
            }
        }
    }

    Ok(true)
}
