use rand::{rngs::ThreadRng, Rng};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{Duration, Instant};

// Default number of passes if not specified.
const DEFAULT_PASSES: usize = 3;

// Machine-readable output schema version.
const MACHINE_SCHEMA: &str = "shredator.machine.v1";

// Define shredding patterns.
#[derive(Clone, Copy, Debug)]
enum ShredPattern {
    Random,      // Random data (default)
    Zeros,       // All zeros
    Ones,        // All ones
    Alternating, // Alternating 0s and 1s
    DoD,         // DoD 5220.22-M style pattern sequence
    Gutmann,     // Peter Gutmann's 35-pass algorithm
}

impl ShredPattern {
    fn as_str(self) -> &'static str {
        match self {
            ShredPattern::Random => "random",
            ShredPattern::Zeros => "zeros",
            ShredPattern::Ones => "ones",
            ShredPattern::Alternating => "alternating",
            ShredPattern::DoD => "dod",
            ShredPattern::Gutmann => "gutmann",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
    JsonLines,
}

impl OutputFormat {
    fn is_machine(self) -> bool {
        matches!(self, OutputFormat::Json | OutputFormat::JsonLines)
    }

    fn as_str(self) -> &'static str {
        match self {
            OutputFormat::Text => "text",
            OutputFormat::Json => "json",
            OutputFormat::JsonLines => "jsonl",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum EventLevel {
    Info,
    Warning,
    Error,
}

impl EventLevel {
    fn as_str(self) -> &'static str {
        match self {
            EventLevel::Info => "info",
            EventLevel::Warning => "warning",
            EventLevel::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunStatus {
    Completed,
    Failed,
    Cancelled,
    Usage,
}

impl RunStatus {
    fn as_str(self) -> &'static str {
        match self {
            RunStatus::Completed => "completed",
            RunStatus::Failed => "failed",
            RunStatus::Cancelled => "cancelled",
            RunStatus::Usage => "usage",
        }
    }

    fn exit_code(self) -> i32 {
        match self {
            RunStatus::Completed => 0,
            RunStatus::Failed => 1,
            RunStatus::Usage => 2,
            RunStatus::Cancelled => 3,
        }
    }
}

#[derive(Debug)]
enum MachineValue {
    String(String),
    U64(u64),
    Usize(usize),
    U128(u128),
    F64(f64),
    Bool(bool),
    Null,
}

impl MachineValue {
    fn to_json(&self) -> String {
        match self {
            MachineValue::String(value) => json_string(value),
            MachineValue::U64(value) => value.to_string(),
            MachineValue::Usize(value) => value.to_string(),
            MachineValue::U128(value) => value.to_string(),
            MachineValue::F64(value) => {
                if value.is_finite() {
                    let mut rendered = format!("{:.6}", value);
                    while rendered.contains('.') && rendered.ends_with('0') {
                        rendered.pop();
                    }
                    if rendered.ends_with('.') {
                        rendered.push('0');
                    }
                    rendered
                } else {
                    "null".to_string()
                }
            }
            MachineValue::Bool(value) => value.to_string(),
            MachineValue::Null => "null".to_string(),
        }
    }
}

#[derive(Debug)]
struct MachineEvent {
    level: EventLevel,
    event: String,
    message: String,
    fields: Vec<(String, MachineValue)>,
}

impl MachineEvent {
    fn new(
        level: EventLevel,
        event: impl Into<String>,
        message: impl Into<String>,
        fields: Vec<(String, MachineValue)>,
    ) -> Self {
        Self {
            level,
            event: event.into(),
            message: message.into(),
            fields,
        }
    }

    fn to_json(&self) -> String {
        let mut parts = Vec::with_capacity(5 + self.fields.len());
        parts.push(format!("{}:{}", json_string("type"), json_string("event")));
        parts.push(format!("{}:{}", json_string("level"), json_string(self.level.as_str())));
        parts.push(format!("{}:{}", json_string("event"), json_string(&self.event)));
        parts.push(format!("{}:{}", json_string("message"), json_string(&self.message)));

        for (key, value) in &self.fields {
            parts.push(format!("{}:{}", json_string(key), value.to_json()));
        }

        format!("{{{}}}", parts.join(","))
    }
}

#[derive(Default, Debug)]
struct RunSummary {
    files_shredded: usize,
    directories_removed: usize,
    paths_successful: usize,
    paths_failed: usize,
    paths_skipped: usize,
    bytes_seen: u64,
    bytes_overwritten: u64,
    overwrite_passes_completed: u64,
    warnings: usize,
    errors: usize,
}

impl RunSummary {
    fn to_json(&self) -> String {
        format!(
            "{{\"files_shredded\":{},\"directories_removed\":{},\"paths_successful\":{},\"paths_failed\":{},\"paths_skipped\":{},\"bytes_seen\":{},\"bytes_overwritten\":{},\"overwrite_passes_completed\":{},\"warnings\":{},\"errors\":{}}}",
            self.files_shredded,
            self.directories_removed,
            self.paths_successful,
            self.paths_failed,
            self.paths_skipped,
            self.bytes_seen,
            self.bytes_overwritten,
            self.overwrite_passes_completed,
            self.warnings,
            self.errors,
        )
    }
}

struct Reporter {
    output_format: OutputFormat,
    verbose: bool,
    quiet: bool,
    events: Vec<MachineEvent>,
    summary: RunSummary,
}

impl Reporter {
    fn new(output_format: OutputFormat, verbose: bool, quiet: bool) -> Self {
        Self {
            output_format,
            verbose,
            quiet,
            events: Vec::new(),
            summary: RunSummary::default(),
        }
    }

    fn set_text_flags(&mut self, verbose: bool, quiet: bool) {
        self.verbose = verbose;
        self.quiet = quiet;
    }

    fn info(&mut self, event: &str, message: impl Into<String>, fields: Vec<(String, MachineValue)>) {
        self.emit(EventLevel::Info, event, message.into(), fields, false);
    }

    fn verbose(&mut self, event: &str, message: impl Into<String>, fields: Vec<(String, MachineValue)>) {
        self.emit(EventLevel::Info, event, message.into(), fields, true);
    }

    fn warning(&mut self, event: &str, message: impl Into<String>, fields: Vec<(String, MachineValue)>) {
        self.summary.warnings += 1;
        self.emit(EventLevel::Warning, event, message.into(), fields, false);
    }

    fn error(&mut self, event: &str, message: impl Into<String>, fields: Vec<(String, MachineValue)>) {
        self.summary.errors += 1;
        self.emit(EventLevel::Error, event, message.into(), fields, false);
    }

    fn emit(
        &mut self,
        level: EventLevel,
        event: &str,
        message: String,
        fields: Vec<(String, MachineValue)>,
        verbose_only: bool,
    ) {
        match self.output_format {
            OutputFormat::Text => {
                if verbose_only && !self.verbose {
                    return;
                }

                match level {
                    EventLevel::Info => {
                        if !self.quiet {
                            println!("{}", message);
                        }
                    }
                    EventLevel::Warning => {
                        if !self.quiet {
                            println!("Warning: {}", message);
                        }
                    }
                    EventLevel::Error => {
                        eprintln!("Error: {}", message);
                    }
                }
            }
            OutputFormat::Json => {
                if verbose_only && !self.verbose {
                    return;
                }
                self.events
                    .push(MachineEvent::new(level, event.to_string(), message, fields));
            }
            OutputFormat::JsonLines => {
                if verbose_only && !self.verbose {
                    return;
                }
                let machine_event = MachineEvent::new(level, event.to_string(), message, fields);
                println!("{}", machine_event.to_json());
            }
        }
    }

    fn finish(&self, status: RunStatus, elapsed: Duration) {
        match self.output_format {
            OutputFormat::Text => {
                if status == RunStatus::Completed {
                    if self.summary.paths_successful > 0
                        || self.summary.files_shredded > 0
                        || self.summary.directories_removed > 0
                    {
                        println!("Shredding completed successfully");
                    }
                }

                if self.quiet || self.summary.paths_successful > 0 || self.summary.paths_failed > 0 || self.summary.paths_skipped > 0 {
                    println!("Summary:");
                    println!("  Successful paths: {}", self.summary.paths_successful);
                    println!("  Failed paths: {}", self.summary.paths_failed);
                    println!("  Skipped paths: {}", self.summary.paths_skipped);
                    println!("  Files shredded: {}", self.summary.files_shredded);
                    println!("  Directories removed: {}", self.summary.directories_removed);
                    println!("  Bytes seen: {}", self.summary.bytes_seen);
                    println!("  Bytes overwritten: {}", self.summary.bytes_overwritten);
                    println!("  Warnings: {}", self.summary.warnings);
                    println!("  Errors: {}", self.summary.errors);
                    println!("  Total time: {:.2} seconds", elapsed.as_secs_f64());
                }
            }
            OutputFormat::Json => {
                let events = self
                    .events
                    .iter()
                    .map(MachineEvent::to_json)
                    .collect::<Vec<_>>()
                    .join(",");

                println!(
                    "{{\"schema\":{},\"type\":{},\"success\":{},\"status\":{},\"exit_code\":{},\"duration_ms\":{},\"output_format\":{},\"summary\":{},\"events\":[{}]}}",
                    json_string(MACHINE_SCHEMA),
                    json_string("summary"),
                    status == RunStatus::Completed,
                    json_string(status.as_str()),
                    status.exit_code(),
                    elapsed.as_millis(),
                    json_string(self.output_format.as_str()),
                    self.summary.to_json(),
                    events
                );
            }
            OutputFormat::JsonLines => {
                println!(
                    "{{\"schema\":{},\"type\":{},\"success\":{},\"status\":{},\"exit_code\":{},\"duration_ms\":{},\"output_format\":{},\"summary\":{}}}",
                    json_string(MACHINE_SCHEMA),
                    json_string("summary"),
                    status == RunStatus::Completed,
                    json_string(status.as_str()),
                    status.exit_code(),
                    elapsed.as_millis(),
                    json_string(self.output_format.as_str()),
                    self.summary.to_json()
                );
            }
        }
    }
}

#[derive(Debug)]
struct Config {
    path_arg: Option<PathBuf>,
    passes: usize,
    verbose: bool,
    quiet: bool,
    force: bool,
    pattern: ShredPattern,
    max_depth: usize,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    benchmark: bool,
    zero_names: bool,
    file_list_path: Option<String>,
    output_format: OutputFormat,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path_arg: None,
            passes: DEFAULT_PASSES,
            verbose: false,
            quiet: false,
            force: false,
            pattern: ShredPattern::Random,
            max_depth: usize::MAX,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            benchmark: false,
            zero_names: false,
            file_list_path: None,
            output_format: OutputFormat::Text,
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let start_time = Instant::now();
    let preliminary_output_format = detect_requested_output_format(&args).unwrap_or(OutputFormat::Text);
    let mut reporter = Reporter::new(preliminary_output_format, false, false);

    let status = match run(&args, &mut reporter) {
        Ok(status) => status,
        Err(error) => {
            reporter.error(
                "fatal_error",
                format!("{}", error),
                vec![("error".to_string(), MachineValue::String(error.to_string()))],
            );
            RunStatus::Failed
        }
    };

    reporter.finish(status, start_time.elapsed());
    process::exit(status.exit_code());
}

fn run(args: &[String], reporter: &mut Reporter) -> io::Result<RunStatus> {
    if args.iter().skip(1).any(|arg| arg == "--help" || arg == "-h") {
        if reporter.output_format == OutputFormat::Text {
            print_usage(&args[0]);
        } else {
            reporter.info(
                "help_requested",
                "Help requested",
                vec![("usage".to_string(), MachineValue::String(format!("{} <file_or_directory_path> [options]", args[0])))],
            );
        }
        return Ok(RunStatus::Completed);
    }

    if args.len() < 2 {
        if reporter.output_format == OutputFormat::Text {
            print_usage(&args[0]);
        } else {
            reporter.error(
                "usage_error",
                "No path specified",
                vec![("reason".to_string(), MachineValue::String("no_path".to_string()))],
            );
        }
        return Ok(RunStatus::Usage);
    }

    let config = match parse_args(args, reporter.output_format) {
        Ok(config) => config,
        Err(message) => {
            if reporter.output_format == OutputFormat::Text {
                eprintln!("Error: {}", message);
                print_usage(&args[0]);
            } else {
                reporter.error(
                    "usage_error",
                    message.clone(),
                    vec![("reason".to_string(), MachineValue::String(message))],
                );
            }
            return Ok(RunStatus::Usage);
        }
    };

    reporter.output_format = config.output_format;
    reporter.set_text_flags(config.verbose, config.quiet);

    if config.path_arg.is_none() && config.file_list_path.is_none() {
        if reporter.output_format == OutputFormat::Text {
            eprintln!("Error: No path specified");
            print_usage(&args[0]);
        } else {
            reporter.error(
                "usage_error",
                "No path specified",
                vec![("reason".to_string(), MachineValue::String("no_path".to_string()))],
            );
        }
        return Ok(RunStatus::Usage);
    }

    if let Some(list_path) = config.file_list_path.as_deref() {
        return process_file_list(
            list_path,
            config.passes,
            config.pattern,
            config.force,
            config.max_depth,
            &config.include_patterns,
            &config.exclude_patterns,
            config.benchmark,
            config.zero_names,
            reporter,
        );
    }

    let path = config.path_arg.as_ref().expect("path checked above");

    if !path.exists() {
        reporter.error(
            "path_not_found",
            format!("Path '{}' does not exist", path.display()),
            vec![("path".to_string(), MachineValue::String(path.display().to_string()))],
        );
        reporter.summary.paths_failed += 1;
        return Ok(RunStatus::Failed);
    }

    if !config.force && (path.is_dir() || is_important_file(path)) {
        if reporter.output_format.is_machine() {
            reporter.warning(
                "confirmation_required",
                format!(
                    "Refusing to continue without --force because '{}' requires confirmation",
                    path.display()
                ),
                vec![
                    ("path".to_string(), MachineValue::String(path.display().to_string())),
                    ("force_required".to_string(), MachineValue::Bool(true)),
                ],
            );
            reporter.summary.paths_skipped += 1;
            return Ok(RunStatus::Cancelled);
        }

        println!("Warning: You are about to permanently destroy {}", path.display());
        println!("This operation cannot be undone. Continue? (y/n)");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            reporter.info(
                "operation_cancelled",
                "Operation cancelled.",
                vec![("path".to_string(), MachineValue::String(path.display().to_string()))],
            );
            reporter.summary.paths_skipped += 1;
            return Ok(RunStatus::Cancelled);
        }
    }

    let result = if path.is_dir() {
        reporter.info(
            "directory_start",
            format!(
                "Recursively shredding directory: {} (using {} passes)",
                path.display(),
                config.passes
            ),
            vec![
                ("path".to_string(), MachineValue::String(path.display().to_string())),
                ("passes".to_string(), MachineValue::Usize(config.passes)),
                ("pattern".to_string(), MachineValue::String(config.pattern.as_str().to_string())),
                ("max_depth".to_string(), MachineValue::Usize(config.max_depth)),
            ],
        );
        shred_directory(
            path,
            config.passes,
            0,
            config.max_depth,
            &config.include_patterns,
            &config.exclude_patterns,
            config.pattern,
            config.benchmark,
            config.zero_names,
            reporter,
        )
    } else {
        reporter.info(
            "file_start_requested",
            format!("Shredding file: {} (using {} passes)", path.display(), config.passes),
            vec![
                ("path".to_string(), MachineValue::String(path.display().to_string())),
                ("passes".to_string(), MachineValue::Usize(config.passes)),
                ("pattern".to_string(), MachineValue::String(config.pattern.as_str().to_string())),
            ],
        );
        shred_file(
            path,
            config.passes,
            config.pattern,
            config.benchmark,
            config.zero_names,
            reporter,
        )
    };

    match result {
        Ok(()) => {
            reporter.summary.paths_successful += 1;
            Ok(RunStatus::Completed)
        }
        Err(error) => {
            reporter.error(
                "operation_failed",
                format!("{}", error),
                vec![
                    ("path".to_string(), MachineValue::String(path.display().to_string())),
                    ("error".to_string(), MachineValue::String(error.to_string())),
                ],
            );
            reporter.summary.paths_failed += 1;
            Ok(RunStatus::Failed)
        }
    }
}

fn detect_requested_output_format(args: &[String]) -> Option<OutputFormat> {
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--json" | "--machine" | "--machine-readable" => return Some(OutputFormat::Json),
            "--jsonl" | "--ndjson" => return Some(OutputFormat::JsonLines),
            "--output" | "--format" => {
                if let Some(value) = args.get(i + 1) {
                    return parse_output_format(value).ok();
                }
                return None;
            }
            _ => i += 1,
        }
    }
    None
}

fn parse_args(args: &[String], detected_output_format: OutputFormat) -> Result<Config, String> {
    let mut config = Config {
        output_format: detected_output_format,
        ..Config::default()
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-p" | "--passes" => {
                let value = require_value(args, i, "--passes")?;
                config.passes = value
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid number of passes: {}", value))?;
                i += 2;
            }
            "-v" | "--verbose" => {
                config.verbose = true;
                i += 1;
            }
            "-q" | "--quiet" => {
                config.quiet = true;
                i += 1;
            }
            "-f" | "--force" => {
                config.force = true;
                i += 1;
            }
            "--pattern" => {
                let value = require_value(args, i, "--pattern")?;
                config.pattern = parse_pattern(value)?;
                if matches!(config.pattern, ShredPattern::Gutmann) {
                    config.passes = 35;
                }
                i += 2;
            }
            "--max-depth" => {
                let value = require_value(args, i, "--max-depth")?;
                config.max_depth = value
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid max depth: {}", value))?;
                i += 2;
            }
            "--include" => {
                let value = require_value(args, i, "--include")?;
                config.include_patterns.push(value.to_string());
                i += 2;
            }
            "--exclude" => {
                let value = require_value(args, i, "--exclude")?;
                config.exclude_patterns.push(value.to_string());
                i += 2;
            }
            "--benchmark" => {
                config.benchmark = true;
                i += 1;
            }
            "--zero-names" => {
                config.zero_names = true;
                i += 1;
            }
            "--file-list" => {
                let value = require_value(args, i, "--file-list")?;
                config.file_list_path = Some(value.to_string());
                i += 2;
            }
            "--json" | "--machine" | "--machine-readable" => {
                config.output_format = OutputFormat::Json;
                i += 1;
            }
            "--jsonl" | "--ndjson" => {
                config.output_format = OutputFormat::JsonLines;
                i += 1;
            }
            "--output" | "--format" => {
                let value = require_value(args, i, "--output")?;
                config.output_format = parse_output_format(value)?;
                i += 2;
            }
            arg if arg.starts_with('-') => {
                return Err(format!("Unknown option: {}", arg));
            }
            _ => {
                if config.path_arg.is_some() {
                    return Err(format!("Multiple paths specified: '{}'", args[i]));
                }
                config.path_arg = Some(PathBuf::from(&args[i]));
                i += 1;
            }
        }
    }

    Ok(config)
}

fn require_value<'a>(args: &'a [String], index: usize, option: &str) -> Result<&'a str, String> {
    args.get(index + 1)
        .map(String::as_str)
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| format!("{} option requires a value", option))
}

fn parse_output_format(value: &str) -> Result<OutputFormat, String> {
    match value.to_lowercase().as_str() {
        "text" | "human" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        "jsonl" | "ndjson" | "lines" => Ok(OutputFormat::JsonLines),
        _ => Err(format!(
            "Unknown output format '{}'. Expected text, json, or jsonl",
            value
        )),
    }
}

fn parse_pattern(value: &str) -> Result<ShredPattern, String> {
    match value.to_lowercase().as_str() {
        "random" => Ok(ShredPattern::Random),
        "zeros" | "zero" => Ok(ShredPattern::Zeros),
        "ones" | "one" => Ok(ShredPattern::Ones),
        "alt" | "alternating" => Ok(ShredPattern::Alternating),
        "dod" => Ok(ShredPattern::DoD),
        "gutmann" => Ok(ShredPattern::Gutmann),
        _ => Err(format!("Unknown pattern: {}", value)),
    }
}

fn print_usage(program: &str) {
    println!("Usage: {} <file_or_directory_path> [options]", program);
    println!("       {} --file-list <path> [options]", program);
    println!("Options:");
    println!(
        "  -p, --passes <number>  Number of overwrite passes (default: {})",
        DEFAULT_PASSES
    );
    println!("  -v, --verbose          Display detailed progress information");
    println!("  -q, --quiet            Only display errors and final summary in text mode");
    println!("  -f, --force            Skip confirmation for sensitive operations");
    println!("  --pattern <type>       Overwrite pattern (random, zeros, ones, alt, dod, gutmann)");
    println!("  --max-depth <number>   Maximum directory depth for recursion");
    println!("  --include <pattern>    Only process files matching pattern (e.g., '*.txt')");
    println!("  --exclude <pattern>    Skip files matching pattern (e.g., '*.jpg')");
    println!("  --benchmark            Measure and report performance statistics");
    println!("  --zero-names           Rename files to random data before deletion");
    println!("  --file-list <path>     Read paths to shred from a text file (one path per line)");
    println!("  --output <format>      Output format: text, json, or jsonl");
    println!("  --json                 Alias for --output json");
    println!("  --jsonl, --ndjson      Alias for --output jsonl");
    println!("  --machine-readable     Alias for --output json");
    println!("");
    println!("Machine-readable mode notes:");
    println!("  * JSON mode prints one final JSON object containing a summary and events.");
    println!("  * JSONL mode prints one JSON object per event, then a final summary object.");
    println!("  * Machine-readable modes do not prompt interactively; use --force when confirmation would be required.");
    println!("");
    println!("Examples:");
    println!("  {} sensitive_file.txt", program);
    println!("  {} sensitive_directory --passes 7 --force", program);
    println!("  {} sensitive_file.txt --force --json", program);
    println!("  {} --file-list targets.txt --force --output jsonl", program);
}

fn shred_directory(
    dir_path: &Path,
    passes: usize,
    current_depth: usize,
    max_depth: usize,
    include_patterns: &[String],
    exclude_patterns: &[String],
    pattern: ShredPattern,
    benchmark: bool,
    zero_names: bool,
    reporter: &mut Reporter,
) -> io::Result<()> {
    if current_depth > max_depth {
        reporter.info(
            "directory_skipped_depth",
            format!("Skipping directory due to depth limit: {}", dir_path.display()),
            vec![
                ("path".to_string(), MachineValue::String(dir_path.display().to_string())),
                ("current_depth".to_string(), MachineValue::Usize(current_depth)),
                ("max_depth".to_string(), MachineValue::Usize(max_depth)),
            ],
        );
        reporter.summary.paths_skipped += 1;
        return Ok(());
    }

    // First collect all entries to avoid borrowing issues during iteration.
    let entries: Vec<_> = fs::read_dir(dir_path)?.collect::<Result<Vec<_>, io::Error>>()?;

    // Process all contents.
    for entry in entries {
        let path = entry.path();

        // Skip if excluded.
        if !exclude_patterns.is_empty()
            && exclude_patterns
                .iter()
                .any(|pattern| matches_pattern(&path, pattern))
        {
            reporter.info(
                "path_skipped_excluded",
                format!("Skipping excluded path: {}", path.display()),
                vec![("path".to_string(), MachineValue::String(path.display().to_string()))],
            );
            reporter.summary.paths_skipped += 1;
            continue;
        }

        // Skip files if includes are specified and the file does not match.
        // Directories are still processed so matching files nested beneath them can be found.
        if !include_patterns.is_empty()
            && !include_patterns
                .iter()
                .any(|pattern| matches_pattern(&path, pattern))
        {
            if !path.is_dir() {
                reporter.info(
                    "path_skipped_not_included",
                    format!("Skipping non-included path: {}", path.display()),
                    vec![("path".to_string(), MachineValue::String(path.display().to_string()))],
                );
                reporter.summary.paths_skipped += 1;
                continue;
            }
        }

        if path.is_dir() {
            shred_directory(
                &path,
                passes,
                current_depth + 1,
                max_depth,
                include_patterns,
                exclude_patterns,
                pattern,
                benchmark,
                zero_names,
                reporter,
            )?;
        } else {
            shred_file(&path, passes, pattern, benchmark, zero_names, reporter)?;
        }
    }

    reporter.info(
        "directory_remove_start",
        format!("Removing directory: {}", dir_path.display()),
        vec![("path".to_string(), MachineValue::String(dir_path.display().to_string()))],
    );

    // Retry directory removal a few times with a small delay.
    // Windows sometimes needs a moment to release file handles.
    let mut retries = 5;
    loop {
        match fs::remove_dir(dir_path) {
            Ok(_) => {
                reporter.summary.directories_removed += 1;
                reporter.info(
                    "directory_removed",
                    format!("Removed directory: {}", dir_path.display()),
                    vec![("path".to_string(), MachineValue::String(dir_path.display().to_string()))],
                );
                break;
            }
            Err(error) => {
                retries -= 1;
                if retries == 0 {
                    return Err(error);
                }
                reporter.verbose(
                    "directory_remove_retry",
                    format!(
                        "Retrying directory removal for {} after error: {}",
                        dir_path.display(),
                        error
                    ),
                    vec![
                        ("path".to_string(), MachineValue::String(dir_path.display().to_string())),
                        ("retries_remaining".to_string(), MachineValue::Usize(retries)),
                        ("error".to_string(), MachineValue::String(error.to_string())),
                    ],
                );
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    Ok(())
}

fn shred_file(
    file_path: &Path,
    passes: usize,
    pattern: ShredPattern,
    benchmark: bool,
    zero_names: bool,
    reporter: &mut Reporter,
) -> io::Result<()> {
    let start_time = Instant::now();

    let metadata = fs::metadata(file_path)?;
    let file_size = metadata.len();
    reporter.summary.bytes_seen = reporter.summary.bytes_seen.saturating_add(file_size);

    reporter.info(
        "file_start",
        format!(
            "Shredding {} ({} bytes) with {} passes",
            file_path.display(),
            file_size,
            passes
        ),
        vec![
            ("path".to_string(), MachineValue::String(file_path.display().to_string())),
            ("size_bytes".to_string(), MachineValue::U64(file_size)),
            ("passes".to_string(), MachineValue::Usize(passes)),
            ("pattern".to_string(), MachineValue::String(pattern.as_str().to_string())),
        ],
    );

    if file_size == 0 {
        reporter.info(
            "empty_file_remove",
            format!("File is empty, removing: {}", file_path.display()),
            vec![("path".to_string(), MachineValue::String(file_path.display().to_string()))],
        );
        fs::remove_file(file_path)?;
        reporter.summary.files_shredded += 1;
        reporter.info(
            "file_removed",
            format!("Removed file: {}", file_path.display()),
            vec![
                ("path".to_string(), MachineValue::String(file_path.display().to_string())),
                ("renamed_before_delete".to_string(), MachineValue::Bool(false)),
            ],
        );
        return Ok(());
    }

    let mut rng = rand::thread_rng();
    let mut completed_passes = 0usize;

    for pass in 1..=passes {
        match overwrite_file_with_pattern(file_path, &mut rng, file_size, pass, passes, pattern, reporter) {
            Ok(()) => {
                completed_passes += 1;
                reporter.summary.overwrite_passes_completed = reporter
                    .summary
                    .overwrite_passes_completed
                    .saturating_add(1);
                reporter.summary.bytes_overwritten = reporter.summary.bytes_overwritten.saturating_add(file_size);
            }
            Err(error) => {
                reporter.warning(
                    "overwrite_pass_failed",
                    format!(
                        "Failed to complete pass {} on file {}: {}",
                        pass,
                        file_path.display(),
                        error
                    ),
                    vec![
                        ("path".to_string(), MachineValue::String(file_path.display().to_string())),
                        ("pass".to_string(), MachineValue::Usize(pass)),
                        ("total_passes".to_string(), MachineValue::Usize(passes)),
                        ("error".to_string(), MachineValue::String(error.to_string())),
                    ],
                );
                // Preserve the original behavior: continue with truncation/deletion even if an overwrite pass fails.
                break;
            }
        }
    }

    // Final pass: truncate to 0 bytes, then delete.
    if let Err(error) = truncate_file(file_path, reporter) {
        reporter.warning(
            "truncate_failed",
            format!("Failed to truncate file {}: {}", file_path.display(), error),
            vec![
                ("path".to_string(), MachineValue::String(file_path.display().to_string())),
                ("error".to_string(), MachineValue::String(error.to_string())),
            ],
        );
    }

    let deleted_path = if zero_names {
        rename_file_before_delete(file_path, &mut rng, reporter)?
    } else {
        file_path.to_path_buf()
    };

    reporter.info(
        "file_remove_start",
        format!("Removing file: {}", deleted_path.display()),
        vec![
            ("path".to_string(), MachineValue::String(deleted_path.display().to_string())),
            (
                "renamed_before_delete".to_string(),
                MachineValue::Bool(zero_names),
            ),
        ],
    );
    fs::remove_file(&deleted_path)?;
    reporter.summary.files_shredded += 1;

    let elapsed = start_time.elapsed();
    reporter.info(
        "file_removed",
        format!("Removed file: {}", deleted_path.display()),
        vec![
            ("path".to_string(), MachineValue::String(deleted_path.display().to_string())),
            ("original_path".to_string(), MachineValue::String(file_path.display().to_string())),
            (
                "renamed_before_delete".to_string(),
                MachineValue::Bool(zero_names),
            ),
            ("completed_passes".to_string(), MachineValue::Usize(completed_passes)),
            ("requested_passes".to_string(), MachineValue::Usize(passes)),
            ("elapsed_ms".to_string(), MachineValue::U128(elapsed.as_millis())),
        ],
    );

    if benchmark {
        let size_mb = file_size as f64 / 1_048_576.0;
        let speed_mbps = if elapsed.as_secs_f64() > 0.0 {
            size_mb * completed_passes as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        reporter.info(
            "benchmark",
            format!(
                "Benchmark results for {}: {:.2} MB in {:.2} seconds ({:.2} MB/s)",
                file_path.display(),
                size_mb,
                elapsed.as_secs_f64(),
                speed_mbps
            ),
            vec![
                ("path".to_string(), MachineValue::String(file_path.display().to_string())),
                ("size_mb".to_string(), MachineValue::F64(size_mb)),
                ("elapsed_seconds".to_string(), MachineValue::F64(elapsed.as_secs_f64())),
                ("throughput_mb_s".to_string(), MachineValue::F64(speed_mbps)),
                ("completed_passes".to_string(), MachineValue::Usize(completed_passes)),
            ],
        );
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum PassFill {
    Random(&'static str),
    Fixed(u8, &'static str),
}

fn pass_fill(pattern: ShredPattern, pass: usize) -> PassFill {
    match pattern {
        ShredPattern::Random => PassFill::Random("Overwriting with random data"),
        ShredPattern::Zeros => PassFill::Fixed(0x00, "Overwriting with zeros"),
        ShredPattern::Ones => PassFill::Fixed(0xFF, "Overwriting with ones"),
        ShredPattern::Alternating => {
            if pass % 2 == 0 {
                PassFill::Fixed(0x55, "Overwriting with alternating bits")
            } else {
                PassFill::Fixed(0xAA, "Overwriting with alternating bits")
            }
        }
        ShredPattern::DoD => match pass % 3 {
            1 => PassFill::Fixed(0x00, "DoD - Overwriting with zeros"),
            2 => PassFill::Fixed(0xFF, "DoD - Overwriting with ones"),
            0 => PassFill::Random("DoD - Overwriting with random data"),
            _ => unreachable!(),
        },
        ShredPattern::Gutmann => match pass {
            1..=4 | 32..=35 => PassFill::Random("Gutmann - Random data"),
            5 => PassFill::Fixed(0x55, "Gutmann - Pattern 1"),
            6 => PassFill::Fixed(0xAA, "Gutmann - Pattern 2"),
            7 => PassFill::Fixed(0x92, "Gutmann - Pattern 3"),
            8 => PassFill::Fixed(0x49, "Gutmann - Pattern 4"),
            9..=31 => {
                let value = ((pass - 9) % 23) as u8;
                PassFill::Fixed(value, "Gutmann - Pattern")
            }
            _ => PassFill::Random("Gutmann - Random data"),
        },
    }
}

fn overwrite_file_with_pattern(
    file_path: &Path,
    rng: &mut ThreadRng,
    file_size: u64,
    pass: usize,
    total_passes: usize,
    pattern: ShredPattern,
    reporter: &mut Reporter,
) -> io::Result<()> {
    let fill = pass_fill(pattern, pass);
    let description = match fill {
        PassFill::Random(description) => description,
        PassFill::Fixed(_, description) => description,
    };

    reporter.info(
        "overwrite_pass_start",
        format!("  Pass {}/{}: {}", pass, total_passes, description),
        vec![
            ("path".to_string(), MachineValue::String(file_path.display().to_string())),
            ("pass".to_string(), MachineValue::Usize(pass)),
            ("total_passes".to_string(), MachineValue::Usize(total_passes)),
            ("pattern".to_string(), MachineValue::String(pattern.as_str().to_string())),
            ("description".to_string(), MachineValue::String(description.to_string())),
        ],
    );

    let mut file = OpenOptions::new().write(true).open(file_path)?;

    // Use a buffer for better performance.
    const BUFFER_SIZE: usize = 8192;
    let mut buffer = vec![0u8; BUFFER_SIZE];

    let full_buffers = file_size / BUFFER_SIZE as u64;
    let remainder = (file_size % BUFFER_SIZE as u64) as usize;

    if let PassFill::Fixed(byte, _) = fill {
        buffer.fill(byte);
    }

    file.seek(SeekFrom::Start(0))?;

    for _ in 0..full_buffers {
        if matches!(fill, PassFill::Random(_)) {
            rng.fill(&mut buffer[..]);
        }
        file.write_all(&buffer)?;
    }

    if remainder > 0 {
        if matches!(fill, PassFill::Random(_)) {
            rng.fill(&mut buffer[..]);
        }
        file.write_all(&buffer[0..remainder])?;
    }

    file.flush()?;
    file.sync_all()?;

    reporter.info(
        "overwrite_pass_complete",
        format!("  Pass {}/{} complete", pass, total_passes),
        vec![
            ("path".to_string(), MachineValue::String(file_path.display().to_string())),
            ("pass".to_string(), MachineValue::Usize(pass)),
            ("total_passes".to_string(), MachineValue::Usize(total_passes)),
            ("bytes_written".to_string(), MachineValue::U64(file_size)),
        ],
    );

    Ok(())
}

fn truncate_file(file_path: &Path, reporter: &mut Reporter) -> io::Result<()> {
    reporter.info(
        "truncate_start",
        "  Final pass: Truncating file to zero bytes",
        vec![("path".to_string(), MachineValue::String(file_path.display().to_string()))],
    );

    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)?;

    file.sync_all()?;

    reporter.info(
        "truncate_complete",
        format!("Truncated file to zero bytes: {}", file_path.display()),
        vec![("path".to_string(), MachineValue::String(file_path.display().to_string()))],
    );

    Ok(())
}

fn rename_file_before_delete(
    file_path: &Path,
    rng: &mut ThreadRng,
    reporter: &mut Reporter,
) -> io::Result<PathBuf> {
    let parent = file_path.parent().unwrap_or(Path::new("."));

    for attempt in 1..=32 {
        let mut random_bytes = [0u8; 8];
        rng.fill(&mut random_bytes[..]);
        let random_name = format!("{:016x}", u64::from_ne_bytes(random_bytes));
        let renamed_path = parent.join(random_name);

        if renamed_path.exists() {
            continue;
        }

        reporter.info(
            "file_rename_start",
            format!("  Renaming file to random name before deletion: {}", renamed_path.display()),
            vec![
                ("path".to_string(), MachineValue::String(file_path.display().to_string())),
                ("renamed_path".to_string(), MachineValue::String(renamed_path.display().to_string())),
                ("attempt".to_string(), MachineValue::Usize(attempt)),
            ],
        );
        fs::rename(file_path, &renamed_path)?;
        reporter.info(
            "file_renamed",
            format!("Renamed file before deletion: {}", renamed_path.display()),
            vec![
                ("original_path".to_string(), MachineValue::String(file_path.display().to_string())),
                ("path".to_string(), MachineValue::String(renamed_path.display().to_string())),
            ],
        );
        return Ok(renamed_path);
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Unable to generate a collision-free random name after 32 attempts",
    ))
}

fn is_important_file(path: &Path) -> bool {
    // Check if the file might be important based on extension or size.
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let important_extensions = ["doc", "docx", "pdf", "xls", "xlsx", "ppt", "pptx", "jpg", "png"];

    if important_extensions.contains(&extension.to_lowercase().as_str()) {
        return true;
    }

    // Check if file is large (>10MB).
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.len() > 10_000_000 {
            return true;
        }
    }

    false
}

fn matches_pattern(path: &Path, pattern: &str) -> bool {
    // Basic glob pattern matching.
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if pattern.starts_with('*') && pattern.ends_with('*') && pattern.len() >= 2 {
        let substr = &pattern[1..pattern.len() - 1];
        file_name.contains(substr)
    } else if let Some(suffix) = pattern.strip_prefix('*') {
        file_name.ends_with(suffix)
    } else if let Some(prefix) = pattern.strip_suffix('*') {
        file_name.starts_with(prefix)
    } else {
        file_name == pattern
    }
}

fn process_file_list(
    list_path: &str,
    passes: usize,
    pattern: ShredPattern,
    force: bool,
    max_depth: usize,
    include_patterns: &[String],
    exclude_patterns: &[String],
    benchmark: bool,
    zero_names: bool,
    reporter: &mut Reporter,
) -> io::Result<RunStatus> {
    let path = Path::new(list_path);
    if !path.exists() {
        reporter.error(
            "file_list_not_found",
            format!("File list not found: {}", list_path),
            vec![("path".to_string(), MachineValue::String(list_path.to_string()))],
        );
        reporter.summary.paths_failed += 1;
        return Ok(RunStatus::Failed);
    }

    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let lines = reader.lines();

    reporter.info(
        "file_list_start",
        format!("Processing paths from file: {}", list_path),
        vec![
            ("path".to_string(), MachineValue::String(list_path.to_string())),
            ("passes".to_string(), MachineValue::Usize(passes)),
            ("pattern".to_string(), MachineValue::String(pattern.as_str().to_string())),
        ],
    );

    for (line_number, line_result) in lines.enumerate() {
        let line_number = line_number + 1;
        match line_result {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    reporter.verbose(
                        "file_list_line_skipped",
                        format!("Skipping line {} (empty or comment)", line_number),
                        vec![("line".to_string(), MachineValue::Usize(line_number))],
                    );
                    reporter.summary.paths_skipped += 1;
                    continue;
                }

                let target_path = Path::new(trimmed);
                if !target_path.exists() {
                    reporter.warning(
                        "file_list_path_missing",
                        format!("Path does not exist (line {}): {}", line_number, trimmed),
                        vec![
                            ("line".to_string(), MachineValue::Usize(line_number)),
                            ("path".to_string(), MachineValue::String(trimmed.to_string())),
                        ],
                    );
                    reporter.summary.paths_skipped += 1;
                    continue;
                }

                if !force && (target_path.is_dir() || is_important_file(target_path)) {
                    reporter.warning(
                        "file_list_path_requires_force",
                        format!(
                            "Skipping important file/directory without --force (line {}): {}",
                            line_number, trimmed
                        ),
                        vec![
                            ("line".to_string(), MachineValue::Usize(line_number)),
                            ("path".to_string(), MachineValue::String(trimmed.to_string())),
                            ("force_required".to_string(), MachineValue::Bool(true)),
                        ],
                    );
                    reporter.summary.paths_skipped += 1;
                    continue;
                }

                reporter.info(
                    "file_list_path_start",
                    format!("Processing path from line {}: {}", line_number, trimmed),
                    vec![
                        ("line".to_string(), MachineValue::Usize(line_number)),
                        ("path".to_string(), MachineValue::String(trimmed.to_string())),
                    ],
                );

                let result = if target_path.is_dir() {
                    shred_directory(
                        target_path,
                        passes,
                        0,
                        max_depth,
                        include_patterns,
                        exclude_patterns,
                        pattern,
                        benchmark,
                        zero_names,
                        reporter,
                    )
                } else {
                    shred_file(target_path, passes, pattern, benchmark, zero_names, reporter)
                };

                match result {
                    Ok(()) => {
                        reporter.summary.paths_successful += 1;
                        reporter.info(
                            "file_list_path_complete",
                            format!("Successfully processed: {}", trimmed),
                            vec![
                                ("line".to_string(), MachineValue::Usize(line_number)),
                                ("path".to_string(), MachineValue::String(trimmed.to_string())),
                            ],
                        );
                    }
                    Err(error) => {
                        reporter.summary.paths_failed += 1;
                        reporter.error(
                            "file_list_path_failed",
                            format!("Error processing path (line {}): {}: {}", line_number, trimmed, error),
                            vec![
                                ("line".to_string(), MachineValue::Usize(line_number)),
                                ("path".to_string(), MachineValue::String(trimmed.to_string())),
                                ("error".to_string(), MachineValue::String(error.to_string())),
                            ],
                        );
                    }
                }
            }
            Err(error) => {
                reporter.summary.paths_failed += 1;
                reporter.error(
                    "file_list_line_read_failed",
                    format!("Error reading line {}: {}", line_number, error),
                    vec![
                        ("line".to_string(), MachineValue::Usize(line_number)),
                        ("error".to_string(), MachineValue::String(error.to_string())),
                    ],
                );
            }
        }
    }

    reporter.info(
        "file_list_complete",
        "File list processing complete",
        vec![
            (
                "successful_paths".to_string(),
                MachineValue::Usize(reporter.summary.paths_successful),
            ),
            ("failed_paths".to_string(), MachineValue::Usize(reporter.summary.paths_failed)),
            ("skipped_paths".to_string(), MachineValue::Usize(reporter.summary.paths_skipped)),
        ],
    );

    if reporter.summary.paths_failed > 0 {
        Ok(RunStatus::Failed)
    } else {
        Ok(RunStatus::Completed)
    }
}

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');

    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0C}' => escaped.push_str("\\f"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }

    escaped.push('"');
    escaped
}
