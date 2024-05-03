/**
 * nbstripout_fast fast is a port of nbstripout_fast to rust. This makes it 3-200x faster
 * in testing. The code is written to nearly match that of
 * https://github.com/kynan/nbstripout_fast/blob/master/nbstripout_fast/_utils.py
 * in order to make it easier to keep feature parity.
 *
 * Note: Not all features were ported (e.g. zeppelin notebooks).
 */
use clap::Parser;
use log;
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use serde_yaml;
use std::env;
use std::fs;
use std::io;
use std::io::BufRead;
use std::path::PathBuf;

mod stripoutlib;

// Match: https://github.com/kynan/nbstripout_fast/blob/master/nbstripout_fast/_nbstripout_fast.py#L431
const DEFAULT_EXTRA_KEYS: [&str; 9] = [
    "metadata.signature",
    "metadata.vscode",
    "metadata.widgets",
    "cell.metadata.collapsed",
    "cell.metadata.ExecuteTime",
    "cell.metadata.execution",
    "cell.metadata.heading_collapsed",
    "cell.metadata.hidden",
    "cell.metadata.scrolled",
];

#[derive(Parser)]
/// Strip output from Jupyter notebooks (modifies the files in place by default).
///
/// This is often used as a git filter when you don't want to track output.
/// By design, this is similar to 'Clear All Output' in Jupyter.
///
/// By default, we load config settings from .git-nbconfig.yaml. CLI options
/// take precedence.
///
/// Usage:
///
/// nbstripout_fast-fast my-notebook.ipynb
///
/// cat my-notebook.ipynb | nbstripout_fast-fast > OUT.ipynb
///
#[clap(author, version, about)]
struct Cli {
    #[clap(long, action)]
    /// Do not strip the execution count/prompt number
    keep_count: bool,

    #[clap(long, action)]
    /// Do not strip output
    keep_output: bool,

    #[clap(long, action)]
    /// Remove cells where `source` is empty or contains only whitespace
    drop_empty_cells: bool,

    #[clap(short, long, action)]
    /// Prints stripped files to STDOUT
    textconv: bool,

    #[clap(short, long, action)]
    /// Space separated list of extra keys to strip
    extra_keys: Option<String>,

    #[clap(short, long, action)]
    /// Space separated list of extra keys NOT to strip (even if in defaults or extra_keys)
    keep_keys: Option<String>,

    #[clap(short, long, action)]
    /// Ignore settings from .git-nbconfig.yaml
    ignore_git_nb_config: bool,

    #[clap(parse(from_os_str))]
    /// Files to strip output from
    files: Vec<PathBuf>,
}

#[derive(Deserialize, Debug)]
struct NBConfigNBStripOutFastConfig {
    keep_output: Option<bool>,
    keep_count: Option<bool>,
    drop_empty_cells: Option<bool>,
    extra_keys: Option<Vec<String>>,
    keep_keys: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
struct NBConfig {
    nbstripout_fast: Option<NBConfigNBStripOutFastConfig>,
}

// TODO: Maybe this should load relative to the file
fn find_nbconfig() -> Result<Option<NBConfig>, String> {
    let mut dir = env::current_dir().map_err(|e| format!("Unable to read current dir {:?}", e))?;
    // Find .git. We don't want too many dependencies, so we do this a bit hacky
    loop {
        let git_dir = dir.as_path().join(".git");
        log::debug!("Looking for .git in {:?}", dir);
        // We don't check if it is a dir as worktrees use a .git file
        if git_dir.exists() {
            let nbconfig_path = dir.join(".git-nbconfig.yaml");
            if nbconfig_path.exists() && nbconfig_path.is_file() {
                let f = std::fs::File::open(nbconfig_path)
                    .map_err(|e| format!("Could not open .git-nbconfig.yaml, {:?}", e))?;
                let config = serde_yaml::from_reader::<std::fs::File, NBConfig>(f)
                    .map_err(|e| format!("Could not parse .git-nbconfig.yaml: {:?}", e))?;
                log::debug!("{:?}", config);
                return Ok(Some(config));
            }

            log::debug!(
                "Could not find {:?}, skipping loading settings from yaml.",
                nbconfig_path
            );
            return Ok(None);
        }
        if let Some(parent) = dir.parent() {
            dir = parent.to_path_buf();
        } else {
            log::debug!("Did not find a git directory, skipping loading yaml config");
            return Ok(None);
        }
    }
}

fn process_file(
    contents: &String,
    keep_output: bool,
    keep_count: bool,
    extra_keys: &Vec<String>,
    drop_empty_cells: bool,
    output_file: Option<PathBuf>,
) -> Result<(), String> {
    let mut nb: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| format!("JSON was not well-formatted: {:?}", e))?;

    stripoutlib::strip_output(
        &mut nb,
        keep_output,
        keep_count,
        &extra_keys,
        drop_empty_cells,
    )?;

    // Format with 1 space to match nbformat
    // https://stackoverflow.com/questions/42722169/generate-pretty-indented-json-with-serde
    let buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
    nb.serialize(&mut ser).map_err(|e| {
        format!(
            "Unable to serialize notebook. Likely an internal error: {:?}",
            e
        )
    })?;
    let mut cleaned_contents = String::from_utf8(ser.into_inner()).map_err(|e| format!("{:?}", e))?;

    // Check if the original content ended with a newline and the cleaned content doesn't
    if contents.ends_with('\n') && !cleaned_contents.ends_with('\n') {
        cleaned_contents.push('\n'); // Append a newline if necessary
    }

    if cleaned_contents != *contents {
        if let Some(file) = output_file {
            fs::write(&file, cleaned_contents)
                .map_err(|e| format!("Could not write to {:?} due to {:?}", file, e))?;
        } else {
            println!("{}", cleaned_contents);
        }
    } else {
        log::debug!("Content unchanged. File not modified.");
    }

    Ok(())
}

fn main() -> Result<(), String> {
    env_logger::init();
    let config = find_nbconfig()?;
    let args = Cli::parse();

    let mut keep_output = false;
    let mut keep_count = false;
    let mut drop_empty_cells = false;

    let mut extra_keys: Vec<String> = vec![];
    for key in DEFAULT_EXTRA_KEYS {
        extra_keys.push(key.to_string());
    }

    // Process config first so that the CLI overrides this
    if let Some(config_yaml) = config {
        if let Some(nbstripout_fast) = config_yaml.nbstripout_fast {
            keep_output = nbstripout_fast.keep_output.unwrap_or(keep_output);
            keep_count = nbstripout_fast.keep_count.unwrap_or(keep_count);
            drop_empty_cells = nbstripout_fast.drop_empty_cells.unwrap_or(drop_empty_cells);
            if let Some(config_extra_keys) = nbstripout_fast.extra_keys {
                for key in config_extra_keys {
                    extra_keys.push(key.to_string());
                }
            }
            if let Some(config_keep_keys) = nbstripout_fast.keep_keys {
                for key in config_keep_keys {
                    // Remove all occurrences
                    extra_keys.retain(|x| x != &key);
                }
            }
        }
    }

    if let Some(extra_keys_str) = args.extra_keys {
        let cli_extra_keys: Vec<String> = extra_keys_str
            .to_owned()
            .split_whitespace()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        for key in cli_extra_keys {
            extra_keys.push(key);
        }
    }
    if let Some(cli_keep_keys_str) = args.keep_keys {
        let cli_keep_keys: Vec<String> = cli_keep_keys_str
            .to_owned()
            .split_whitespace()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        for key in cli_keep_keys {
            // Remove all occurances
            extra_keys.retain(|x| x != &key);
        }
    }
    if args.keep_output {
        keep_output = true;
    }
    if args.keep_count {
        keep_count = true;
    }
    if args.drop_empty_cells {
        drop_empty_cells = true;
    }

    log::debug!(
        "Using keep_count: {} keep_output: {} drop_empty_cells: {} extra_keys: {:?} ",
        keep_count,
        keep_output,
        drop_empty_cells,
        extra_keys
    );
    if args.files.is_empty() {
        log::debug!("Processing stdin");
        let contents: String = io::stdin().lock().lines().map(|ln| ln.unwrap()).collect();
        process_file(
            &contents,
            keep_output,
            keep_count,
            &extra_keys,
            drop_empty_cells,
            None,
        )?;
    } else {
        for file in args.files {
            // Much faster than using from_reader for some reason - https://github.com/serde-rs/json/issues/160
            log::debug!("Processing file {:?}", file);
            let contents = fs::read_to_string(&file)
                .map_err(|e| format!("Could not load {:?}: {:?}", file, e.to_string()))?;

            let output_file = match args.textconv {
                false => Some(file),
                true => None,
            };

            process_file(
                &contents,
                keep_output,
                keep_count,
                &extra_keys,
                drop_empty_cells,
                output_file,
            )?;
        }
    }

    Ok(())
}
