use clap::{command, Subcommand, Parser, ValueEnum};
use indicatif::ProgressStyle;
use log::LevelFilter;

#[derive(Parser, Clone, Debug)]
pub struct Cli {
    #[command(subcommand)]
    pub sub_command: CliSubCommand,
    #[clap(env, long, default_value = "Info")]
    pub rust_log: LogLevel,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CliSubCommand {
    Flame {
        #[clap(env, long)]
        api_key: String,
        #[clap(env, long)]
        project_id: u64,
        #[clap(env, long)]
        version: String,
        #[clap(env, long)]
        target_dir: String,
    },
    Ftb {
        #[clap(env, long,  required_unless_present="id", conflicts_with="id")]
        search_terms: Option<Vec<String>>,
        #[clap(env, long, required_unless_present="search_terms")]
        id: Option<String>,
        #[clap(env, long, requires="search_terms")]
        mc_version: Option<String>,
        #[clap(env, long)]
        version: String,
        #[clap(env, long)]
        target_dir: String,
    },
    Forge {
        #[clap(env, long)]
        mc_version: String,
        #[clap(env, long)]
        version: String,
        #[clap(env, long)]
        target_dir: String,
    },
    Fabric {
        #[clap(env, long)]
        mc_version: String,
        #[clap(env, long)]
        version: String,
        #[clap(env, long)]
        target_dir: String,
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Off => LevelFilter::Off,
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Trace => LevelFilter::Trace,
        }
    }
}

pub fn download_progress_style() -> ProgressStyle {
    ProgressStyle::with_template("[File: {msg}]\n{bar:40.cyan/blue} {percent}% [{bytes} / {total_bytes}] [Eta: {eta}]").unwrap()
}

pub fn copy_progress_style() -> ProgressStyle {
    ProgressStyle::with_template("{prefix.bold.dim} {spinner} {msg} {elapsed}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
}

pub fn backup_progress_style() -> ProgressStyle {
    ProgressStyle::with_template("{bar:40.cyan/blue} [Eta: {eta}]\n[{pos}/{len}] {wide_msg}")
        .unwrap()
}

pub fn spinner_progress_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner} {prefix} {elapsed}\n{wide_msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
}