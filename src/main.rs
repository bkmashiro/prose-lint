use prose_lint::{Format, Profile, Report, ScanOptions, Scanner};
use rayon::prelude::*;
use std::env;
use std::fs;
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
    --jobs <N>         Maximum parallel file scans
    -h, --help         Print help
    -V, --version      Print version

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
    let scanner = Scanner::builtin().map_err(|error| error.to_string())?;
    let mut files = Vec::new();
    for path in &cli.paths {
        collect_files(path, &mut files)?;
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err("no supported prose files found".to_owned());
    }

    let options = ScanOptions {
        profile: cli.profile,
        show_all: cli.show_all,
    };
    let scan = || {
        files
            .par_iter()
            .map(|path| {
                let text = fs::read_to_string(path)
                    .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
                Ok(scanner.scan_text(&path.display().to_string(), &text, &options))
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

    match cli.format {
        Format::Json => println!(
            "{}",
            serde_json::to_string_pretty(&reports)
                .map_err(|error| format!("cannot render JSON: {error}"))?
        ),
        Format::Text => {
            for report in &reports {
                print!(
                    "{}",
                    report
                        .render(Format::Text)
                        .map_err(|error| error.to_string())?
                );
            }
        }
    }

    let should_fail = cli.strict
        && reports
            .iter()
            .any(|report| report.high_confidence_count() > 0);
    Ok(if should_fail {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn parse_args(args: Vec<String>) -> Result<Option<Cli>, String> {
    if args.is_empty() {
        print!("{HELP}");
        return Ok(None);
    }
    let mut paths = Vec::new();
    let mut profile = Profile::Technical;
    let mut format = Format::Text;
    let mut show_all = false;
    let mut strict = false;
    let mut jobs = None;
    let mut index = usize::from(args.first().is_some_and(|arg| arg == "scan"));

    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("prose-lint {}", env!("CARGO_PKG_VERSION"));
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
    }))
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
