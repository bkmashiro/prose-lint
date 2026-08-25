use glob::{MatchOptions, glob_with};
use prose_lint::{CustomTerm, Format, Profile, Report, ScanOptions, Scanner, Severity};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HELP: &str = r#"prose-lint 0.1.0
Fast, evidence-aware linter for formulaic LLM-shaped prose.

USAGE:
    prose-lint [scan] [OPTIONS] <PATH>...

OPTIONS:
    --profile <NAME>   technical, academic, pr, commit, casual, marketing
    --format <FORMAT>  text or json [default: text]
    --all              Show low-confidence empirical vocabulary findings
    --strict           Exit 1 when a high-confidence finding is present
    --config <PATH>    Use one config for every input instead of auto-discovery
    --jobs <N>         Maximum parallel file scans
    -h, --help         Print help
    -V, --version      Print version

PATH accepts shell-style glob patterns. Quote recursive patterns so the CLI,
rather than the shell, expands them: prose-lint scan '**/*.typ'

The output is a writing aid, not an AI-authorship detector.
"#;

#[derive(Debug)]
struct Cli {
    paths: Vec<PathBuf>,
    profile: Profile,
    format: Format,
    show_all: bool,
    strict: bool,
    jobs: Option<usize>,
    config: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepoConfig {
    #[serde(default)]
    extra_terms: Vec<ExtraTermConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ExtraTermConfig {
    Simple(String),
    Detailed(DetailedTermConfig),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DetailedTermConfig {
    term: String,
    #[serde(default = "default_custom_severity")]
    severity: Severity,
    message: Option<String>,
    suggestion: Option<String>,
}

fn default_custom_severity() -> Severity {
    Severity::Medium
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("prose-lint: {message}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let Some(cli) = parse_args(env::args().skip(1).collect())? else {
        return Ok(ExitCode::SUCCESS);
    };
    let mut files = Vec::new();
    for path in &cli.paths {
        collect_input(path, &mut files)?;
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err("no supported prose files found".to_owned());
    }

    let mut scanners = vec![Scanner::builtin().map_err(|error| error.to_string())?];
    let mut scanner_indices = HashMap::new();
    let mut config_discovery = HashMap::new();
    let mut scan_jobs = Vec::with_capacity(files.len());
    for path in files {
        let config_path = match &cli.config {
            Some(config) => Some(config.clone()),
            None => find_repo_config(&path, &mut config_discovery)?,
        };
        let scanner_index = match config_path {
            Some(config_path) => {
                if let Some(index) = scanner_indices.get(&config_path) {
                    *index
                } else {
                    let terms = load_custom_terms(&config_path)?;
                    let scanner = Scanner::builtin_with_custom_terms(&terms).map_err(|error| {
                        format!("cannot compile {}: {error}", config_path.display())
                    })?;
                    let index = scanners.len();
                    scanners.push(scanner);
                    scanner_indices.insert(config_path, index);
                    index
                }
            }
            None => 0,
        };
        scan_jobs.push((path, scanner_index));
    }

    let options = ScanOptions {
        profile: cli.profile,
        show_all: cli.show_all,
    };
    let scan =
        || {
            scan_jobs
                .par_iter()
                .map(|(path, scanner_index)| {
                    let text = fs::read_to_string(path)
                        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
                    Ok(scanners[*scanner_index].scan_text(
                        &path.display().to_string(),
                        &text,
                        &options,
                    ))
                })
                .collect::<Vec<Result<Report, String>>>()
        };
    let results = if let Some(jobs) = cli.jobs {
        if jobs == 0 {
            return Err("--jobs must be at least 1".to_owned());
        }
        rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build()
            .map_err(|error| format!("cannot build worker pool: {error}"))?
            .install(scan)
    } else {
        scan()
    };
    let mut reports = results.into_iter().collect::<Result<Vec<_>, _>>()?;
    reports.sort_by(|left, right| left.path.cmp(&right.path));

    let should_fail = cli.strict
        && reports
            .iter()
            .any(|report| report.high_confidence_count() > 0);
    let exit_code = if should_fail {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    };
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    match cli.format {
        Format::Json => {
            let mut rendered = serde_json::to_string_pretty(&reports)
                .map_err(|error| format!("cannot render JSON: {error}"))?;
            rendered.push('\n');
            if !write_output(&mut stdout, rendered.as_bytes())? {
                return Ok(exit_code);
            }
        }
        Format::Text => {
            for report in &reports {
                let rendered = report
                    .render(Format::Text)
                    .map_err(|error| error.to_string())?;
                if !write_output(&mut stdout, rendered.as_bytes())? {
                    return Ok(exit_code);
                }
            }
        }
    }
    Ok(exit_code)
}

fn write_output(writer: &mut impl Write, bytes: &[u8]) -> Result<bool, String> {
    match writer.write_all(bytes) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(false),
        Err(error) => Err(format!("cannot write output: {error}")),
    }
}

fn parse_args(args: Vec<String>) -> Result<Option<Cli>, String> {
    if args.is_empty() {
        let stdout = io::stdout();
        write_output(&mut stdout.lock(), HELP.as_bytes())?;
        return Ok(None);
    }
    let mut paths = Vec::new();
    let mut profile = Profile::Technical;
    let mut format = Format::Text;
    let mut show_all = false;
    let mut strict = false;
    let mut jobs = None;
    let mut config = None;
    let mut index = usize::from(args.first().is_some_and(|arg| arg == "scan"));

    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => {
                let stdout = io::stdout();
                write_output(&mut stdout.lock(), HELP.as_bytes())?;
                return Ok(None);
            }
            "-V" | "--version" => {
                let version = format!("prose-lint {}\n", env!("CARGO_PKG_VERSION"));
                let stdout = io::stdout();
                write_output(&mut stdout.lock(), version.as_bytes())?;
                return Ok(None);
            }
            "--all" => show_all = true,
            "--strict" => strict = true,
            "--profile" => {
                index += 1;
                let value = args.get(index).ok_or("--profile requires a value")?;
                profile = Profile::parse(value).ok_or_else(|| {
                    format!("unknown profile {value:?}; use technical, academic, pr, commit, casual, or marketing")
                })?;
            }
            "--format" => {
                index += 1;
                let value = args.get(index).ok_or("--format requires a value")?;
                format = match value.as_str() {
                    "text" => Format::Text,
                    "json" => Format::Json,
                    _ => return Err(format!("unknown format {value:?}; use text or json")),
                };
            }
            "--jobs" => {
                index += 1;
                let value = args.get(index).ok_or("--jobs requires a value")?;
                jobs = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| format!("invalid job count {value:?}"))?,
                );
            }
            "--config" => {
                index += 1;
                config = Some(PathBuf::from(
                    args.get(index).ok_or("--config requires a value")?,
                ));
            }
            "--" => {
                paths.extend(args[index + 1..].iter().map(PathBuf::from));
                break;
            }
            value if value.starts_with('-') => return Err(format!("unknown option {value:?}")),
            value => paths.push(PathBuf::from(value)),
        }
        index += 1;
    }

    if paths.is_empty() {
        return Err("at least one path is required".to_owned());
    }
    Ok(Some(Cli {
        paths,
        profile,
        format,
        show_all,
        strict,
        jobs,
        config,
    }))
}

fn find_repo_config(
    path: &Path,
    cache: &mut HashMap<PathBuf, Option<PathBuf>>,
) -> Result<Option<PathBuf>, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|error| format!("cannot read current directory: {error}"))?
            .join(path)
    };
    let mut directory = absolute.parent().map(Path::to_path_buf);
    let mut visited = Vec::new();
    let result = loop {
        let Some(current) = directory else {
            break None;
        };
        if let Some(config) = cache.get(&current) {
            break config.clone();
        }
        visited.push(current.clone());
        let config = current.join(".prose-lint.json");
        if config.is_file() {
            break Some(config);
        }
        if current.join(".git").exists() {
            break None;
        }
        directory = current.parent().map(Path::to_path_buf);
    };
    for directory in visited {
        cache.insert(directory, result.clone());
    }
    Ok(result)
}

fn load_custom_terms(path: &Path) -> Result<Vec<CustomTerm>, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let config: RepoConfig = serde_json::from_str(&source)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    config
        .extra_terms
        .into_iter()
        .map(|entry| {
            let (term, severity, message, suggestion) = match entry {
                ExtraTermConfig::Simple(term) => (term, Severity::Medium, None, None),
                ExtraTermConfig::Detailed(detail) => (
                    detail.term,
                    detail.severity,
                    detail.message,
                    detail.suggestion,
                ),
            };
            let term = term.trim().to_owned();
            if term.is_empty() {
                return Err(format!(
                    "invalid {}: custom term must not be empty",
                    path.display()
                ));
            }
            Ok(CustomTerm {
                message: message.unwrap_or_else(|| {
                    format!("{term:?} is discouraged by this repository's prose policy.")
                }),
                suggestion: suggestion.unwrap_or_else(|| {
                    "Use the repository's preferred wording or explain why this term is needed."
                        .to_owned()
                }),
                term,
                severity,
            })
        })
        .collect()
}

fn collect_input(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let pattern = path.to_string_lossy();
    if path.exists() || !contains_glob_meta(&pattern) {
        return collect_files(path, files);
    }

    let options = MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: true,
    };
    let entries = glob_with(&pattern, options)
        .map_err(|error| format!("invalid glob pattern {pattern:?}: {error}"))?;
    let literal_root = literal_glob_root(path);
    let mut matched = false;
    for entry in entries {
        let matched_path = entry.map_err(|error| format!("glob traversal error: {error}"))?;
        matched = true;
        if path_below_root_has_symlink_component(&matched_path, &literal_root)? {
            continue;
        }
        collect_files(&matched_path, files)?;
    }
    if !matched {
        return Err(format!("pattern matched no paths: {pattern}"));
    }
    Ok(())
}

fn literal_glob_root(pattern: &Path) -> PathBuf {
    let mut root = PathBuf::new();
    for component in pattern.components() {
        let value = component.as_os_str().to_string_lossy();
        if contains_glob_meta(&value) {
            break;
        }
        root.push(component.as_os_str());
    }
    if root.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        root
    }
}

fn path_below_root_has_symlink_component(path: &Path, root: &Path) -> Result<bool, String> {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("cannot inspect {}: {error}", current.display()))?;
        if metadata.file_type().is_symlink() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn contains_glob_meta(pattern: &str) -> bool {
    pattern
        .bytes()
        .any(|byte| matches!(byte, b'*' | b'?' | b'['))
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    if metadata.is_file() {
        if is_supported(path) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    if !metadata.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(path)
        .map_err(|error| format!("cannot read directory {}: {error}", path.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("directory entry error: {error}"))?;
        let child = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("cannot inspect {}: {error}", child.display()))?;
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if file_type.is_dir()
            && (name.starts_with('.')
                || matches!(
                    name.as_ref(),
                    "target" | "node_modules" | "vendor" | "dist" | "build"
                ))
        {
            continue;
        }
        collect_files(&child, files)?;
    }
    Ok(())
}

fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "md" | "mdx" | "txt" | "rst" | "adoc" | "tex" | "typ"
            )
        })
}
