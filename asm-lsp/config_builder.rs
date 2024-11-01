use std::path::Path;
use std::string::ToString;
use std::{env::current_dir, path::PathBuf};

use anyhow::{anyhow, Result};
use clap::{arg, Args};
use dirs::config_dir;

use crate::types::{Arch, Assembler, Config, ConfigOptions, ProjectConfig, RootConfig};

use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input};

const ARCH_LIST: [Arch; 7] = [
    Arch::X86,
    Arch::X86_64,
    Arch::X86_AND_X86_64,
    Arch::ARM,
    Arch::ARM64,
    Arch::RISCV,
    Arch::Z80,
];

const ASSEMBLER_LIST: [Assembler; 4] = [
    Assembler::Gas,
    Assembler::Go,
    Assembler::Masm,
    Assembler::Nasm,
];

#[derive(Args, Debug, Clone)]
#[command(about = "Generate a .asm-lsp.toml config file")]
pub struct GenerateArgs {
    #[arg(
        long,
        short,
        help = "Directory to place .asm-lsp.toml into. (Default is the current directory)"
    )]
    pub output_dir: Option<PathBuf>,
    #[arg(
        long,
        short,
        conflicts_with = "output_dir",
        help = "Place the config in the global config directory"
    )]
    pub global_cfg: bool,
    #[arg(
        long,
        short,
        conflicts_with = "global_cfg",
        help = "Path to the project this config is being generated for. (Default is the current directory)"
    )]
    pub project_path: Option<PathBuf>,
    #[arg(
        short = 'w',
        long,
        help = "Overwrite any existing .asm-lsp.toml in the target directory"
    )]
    pub overwrite: bool,
    #[arg(
        short,
        long,
        help = "Don't display the generated config file after generation"
    )]
    pub quiet: bool,
}

#[derive(Debug, Clone)]
pub struct GenerateOpts {
    pub output_path: PathBuf,
    pub project_path: PathBuf,
    pub overwrite: bool,
    pub quiet: bool,
}

impl TryFrom<GenerateArgs> for GenerateOpts {
    type Error = String;
    fn try_from(value: GenerateArgs) -> Result<Self, std::string::String> {
        let output_path = {
            if value.global_cfg {
                let mut path = config_dir().ok_or_else(|| "Failed to detect config directory, try specifying it manually with `--output_dir`".to_string())?;
                path.push("asm-lsp");
                path.push(".asm-lsp.toml");
                path
            } else if let Some(path) = value.output_dir.as_ref() {
                let mut canonicalized_path = path.canonicalize().map_err(|e| {
                    format!(
                        "Failed to canonicalize target path: \"{}\" -- {e}",
                        path.display()
                    )
                })?;
                if !canonicalized_path.is_dir() {
                    let gave_file_name = canonicalized_path.ends_with(".asm-lsp.toml");
                    return Err(format!(
                        "Target path \"{}\" is not a directory.{}",
                        canonicalized_path.display(),
                        if gave_file_name {
                            "Hint: Don't include the filename \".asm-lsp.toml\" at the end of your target path."
                        } else {
                            ""
                        }
                    ));
                }
                canonicalized_path.push(".asm-lsp.toml");
                canonicalized_path
            } else {
                let mut path = current_dir()
                    .map_err(|e| format!("Failed to detect current directory -- {e}"))?;
                path.push(".asm-lsp.toml");
                path
            }
        };
        let project_path = {
            if let Some(path) = value.project_path.as_ref().or(value.output_dir.as_ref()) {
                let canonicalized_path = path.canonicalize().map_err(|e| {
                    format!(
                        "Failed to canonicalize project path: \"{}\" -- {e}",
                        path.display()
                    )
                })?;
                if !canonicalized_path.is_dir() {
                    return Err(format!(
                        "Project path \"{}\" is not a directory.",
                        canonicalized_path.display(),
                    ));
                }
                canonicalized_path
            } else {
                current_dir().map_err(|e| format!("Failed to detect current directory -- {e}"))?
            }
        };

        Ok(Self {
            output_path,
            project_path,
            overwrite: value.overwrite,
            quiet: value.quiet,
        })
    }
}

fn prompt_arch() -> Arch {
    let arch_choices: Vec<String> = ARCH_LIST.iter().map(ToString::to_string).collect();
    let arch_selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select architecture")
        .default(0)
        .items(&arch_choices[..])
        .interact()
        .unwrap();

    ARCH_LIST[arch_selection]
}

fn prompt_assembler() -> Assembler {
    let assem_choices: Vec<String> = ASSEMBLER_LIST.iter().map(ToString::to_string).collect();
    let assem_selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select assembler")
        .default(0)
        .items(&assem_choices[..])
        .interact()
        .unwrap();

    ASSEMBLER_LIST[assem_selection]
}

fn prompt_project_path(opts: &GenerateOpts) -> PathBuf {
    println!("Provide a project path:");
    let fallback_enter = |true_path: &mut PathBuf| {
        println!(
            "Warning: Failed to create directory reader for path \"{}\"",
            true_path.display()
        );
        let remaining_path: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter remaining path (Enter an empty string to use the current path)")
            .allow_empty(true)
            .interact_text()
            .unwrap();
        true_path.push(remaining_path);
    };
    let mut true_path = opts.project_path.clone();
    let mut display_entries = Vec::new();
    let mut path_entries = Vec::new();
    loop {
        let selection_text = format!("{}", true_path.display());
        // Dummy entry to account for the accept option as the first displayed
        // option
        path_entries.push(PathBuf::new());
        display_entries.push("<Select This Directory>".to_string());
        let Ok(dir_reader) = std::fs::read_dir(&true_path) else {
            fallback_enter(&mut true_path);
            return true_path;
        };

        let mut dir_entries = Vec::new();
        let mut file_entries = Vec::new();
        for entry in dir_reader.filter_map(std::result::Result::ok) {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                dir_entries.push(entry_path);
            } else {
                file_entries.push(entry_path);
            }
        }

        dir_entries.sort();
        file_entries.sort();

        for entry_path in dir_entries.into_iter().chain(file_entries) {
            path_entries.push(entry_path.clone());
            if let Some(name) = entry_path.file_name() {
                display_entries.push(name.to_string_lossy().to_string());
            }
        }
        let path_selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt(&selection_text)
            .default(0)
            .items(&display_entries[..])
            .interact()
            .unwrap();

        // Select current value of `true_path`
        if path_selection == 0 {
            if true_path.to_string_lossy().len() == opts.project_path.to_string_lossy().len() &&
                !Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Warning: Creating project config for the entire project. Keep this path selection?")
                    .default(false)
                    .interact()
                    .unwrap() {
                       continue;
            }
            break;
        }

        true_path.clone_from(&path_entries[path_selection]);
        if true_path.is_file() {
            break;
        }
        path_entries.clear();
        display_entries.clear();
    }

    // Get the project-relative path out for the sake of portability, i.e. if a
    // users changes the location of their project, their config should still work
    let mut relative_path = PathBuf::new();
    for comp in true_path
        .components()
        .skip(opts.project_path.components().count())
    {
        relative_path.push(comp);
    }

    relative_path
}

fn prompt_project(opts: &GenerateOpts) -> ProjectConfig {
    let path = prompt_project_path(opts);
    let config = prompt_config();

    ProjectConfig { path, config }
}

/// Check if a path points to an executable file
fn is_executable(path: &Path) -> bool {
    if path.is_file() {
        #[cfg(unix)]
        {
            // On Unix, check the `x` bit
            use std::fs;
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(path).unwrap();
            metadata.permissions().mode() & 0o111 != 0
        }
        #[cfg(windows)]
        {
            // On Windows, check for common executable extensions
            let extensions = ["exe", "cmd", "bat", "com"];
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                return extensions.contains(&ext);
            }
            false
        }
    } else {
        false
    }
}

/// Check if `cmd` has a corresponding executable file on $PATH
#[must_use]
fn is_executable_on_path(cmd: &str) -> bool {
    use std::env;
    // Get the PATH environment variable
    let path_var = env::var_os("PATH").unwrap();

    for path in env::split_paths(&path_var) {
        let full_path = path.join(cmd);
        if is_executable(&full_path) {
            return true;
        }
    }
    println!("Warning: Unable to find provided compiler as executable file on $PATH");
    false
}

fn validate_compiler(comp: &str) -> bool {
    // Attempt to provide some soft validation, warn the user if something
    // looks fishy
    if comp.contains(std::path::MAIN_SEPARATOR) {
        // Treat it as a path
        let Ok(path) = PathBuf::from(comp).canonicalize() else {
            println!("Warning: Failed to canonicalize path \"{comp}\"",);
            return false;
        };
        let exists = path.exists();
        let is_file = path.is_file();
        let is_exec = is_executable(&path);
        if !exists {
            println!(
                "Warning: File does not exist at path \"{}\"",
                path.display()
            );
        } else if !is_file {
            println!(
                "Warning: Path \"{}\" does not point to a file",
                path.display()
            );
        } else if !is_exec {
            println!(
                "Warning: Path \"{}\" does not point to an executable file",
                path.display()
            );
        }

        exists && is_file && is_exec
    } else {
        is_executable_on_path(comp)
    }
}

fn prompt_compiler() -> Option<String> {
    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Provide compiler to use with `compile_flags.txt` files or the following (optional) compile flags field")
        .default(true)
        .interact()
        .unwrap() {
        return None;
    }
    let mut comp: String;
    loop {
        comp = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter Compiler")
            .interact_text()
            .unwrap();

        // Attempt to provide some soft validation, warn the user if something
        // looks fishy
        if validate_compiler(&comp)
            || !Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Re-enter compiler?")
                .default(true)
                .interact()
                .unwrap()
        {
            break;
        }
    }

    Some(comp)
}

fn prompt_config_opts() -> ConfigOptions {
    let compiler = prompt_compiler();
    let compile_flags_txt = if compiler.is_some() {
        let mut flags = Vec::new();
        loop {
            let flag: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Add a compiler flag: (Enter an empty string to stop)")
                .allow_empty(true)
                .validate_with(|input: &String| -> Result<()> {
                    // NOTE: Do we need to handle escaped quotes here?
                    let mut in_quotes = false;
                    for (i, c) in input.chars().enumerate() {
                        match c {
                            '\"' => in_quotes = !in_quotes,
                            ' ' => {
                                if !in_quotes {
                                    return Err(anyhow!(
                                        "\n{input}\n{}^\nUnquoted space found, specify each flag separately.",
                                        " ".repeat(i),
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(())
                })
                .interact_text()
                .unwrap();
            if flag.is_empty() {
                break;
            }
            flags.push(flag);
        }
        Some(flags)
    } else {
        None
    };

    let diagnostics = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Enable diagnostic features?")
        .default(true)
        .interact()
        .unwrap();

    // only offer the `default_diagnostics` option if both
    //      A) diagnostics are enabled
    //      B) The user didn't specify compiler instructions via the `compiler`
    //         field (if they did so, default diagnostics will never be used)
    let default_diagnostics = if diagnostics && compiler.is_none() && Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Attempt to provide diagnostics if no compilation information can be found for a source file?")
        .default(true)
        .interact()
        .unwrap() {
            Some(true)
    } else {
        Some(false)
    };

    ConfigOptions {
        compiler,
        compile_flags_txt,
        diagnostics: Some(diagnostics),
        default_diagnostics,
    }
}

fn prompt_config() -> Config {
    let instruction_set = prompt_arch();
    let assembler = prompt_assembler();
    let opts = if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Configure diagnostic related features?")
        .default(true)
        .interact()
        .unwrap()
    {
        Some(prompt_config_opts())
    } else {
        None
    };

    Config {
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
        instruction_set,
        assembler,
        opts,
        client: None,
    }
}

fn prompt_root_config(opts: &GenerateOpts) -> RootConfig {
    let default_config = if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Create default config?")
        .interact()
        .unwrap()
    {
        Some(prompt_config())
    } else {
        None
    };

    let mut projects = Vec::new();
    loop {
        if !Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Add a new project config?")
            .interact()
            .unwrap()
        {
            break;
        }
        projects.push(prompt_project(opts));
    }

    RootConfig {
        default_config,
        projects: if projects.is_empty() {
            None
        } else {
            Some(projects)
        },
    }
}

/// Prompts the user through generating a `.asm-lsp.toml` config file
///
/// # Errors
///
/// Returns `Err` if
///     - A config file exists and the `--overwrite` flag wasn't used
///     - An error occurs during config generation
///     - Serialization of the generated config fails
///     - Writing the serialized config to a file fails
pub fn gen_config(opts: &GenerateOpts) -> Result<()> {
    if !opts.overwrite && opts.output_path.exists() {
        return Err(anyhow!(
            "The target path \"{}\" already exists and `--overwrite` was not used",
            opts.output_path.display()
        ));
    }
    let root_config = prompt_root_config(opts);
    let file_config = toml::to_string_pretty::<RootConfig>(&root_config).map_err(|e| {
        anyhow!("Failed to serialize configuration -- {e}\nPlease file a bug report: https://github.com/bergercookie/asm-lsp/issues/new")
    })?;
    if !opts.quiet {
        println!("{file_config}");
    }
    std::fs::write(&opts.output_path, file_config).map_err(|e| {
        anyhow!(
            "Failed to write config file to path \"{}\" -- {e}",
            opts.output_path.display()
        )
    })?;
    Ok(())
}
