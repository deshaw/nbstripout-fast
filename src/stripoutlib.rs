// This code is nearly a 1:1 mapping of https://github.com/kynan/nbstripout/blob/master/nbstripout/_utils.py
use log;
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

// Should we keep the output of a given cell?
// If the cell contains keep_output in the metadata or tag metadata
fn determine_keep_output(cell: &JSONMap, default: bool) -> Result<bool, String> {
    if !cell.contains_key("metadata") {
        return Ok(default);
    }
    let metadata = match cell["metadata"].as_object() {
        Some(x) => x,
        None => return Ok(default),
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

    if has_keep_output_metadata || has_keep_output_tag {
        return Ok(keep_output_metadata || has_keep_output_tag);
    }

    Ok(default)
}

// TODO: add custom errors instead of returning a string
#[cfg_attr(not(feature = "extension-module"), allow(unused))]
pub fn strip_output(
    nb: &mut serde_json::Value,
    keep_output: bool,
    keep_count: bool,
    extra_keys: &Vec<String>,
    drop_empty_cells: bool,
) -> Result<bool, String> {
    log::debug!(
        "keep-output: {}, keep-count: {}, extra-keys: {:?}, drop-empty-cells: {}",
        keep_output,
        keep_count,
        extra_keys,
        drop_empty_cells
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

        // Clean up each cell as required
        for cell_object in cells {
            if !cell_object.is_object() {
                log::debug!("Skipping non-object cell");
                continue;
            }
            let cell = cell_object.as_object_mut().expect("Cell must be an object");
            let keep_output_this_cell_option = determine_keep_output(cell, keep_output);
            let keep_output_this_cell = keep_output_this_cell_option?;

            if cell.contains_key("outputs") {
                let outputs = cell["outputs"]
                    .as_array_mut()
                    .expect("Outputs must be an array");
                //  Default behavior (max_size == 0) strips all outputs.
                if !keep_output_this_cell {
                    outputs.clear();
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
