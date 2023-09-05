use clap::{command, Subcommand, Parser};

#[derive(Parser, Clone, Debug)]
pub struct Cli {
    #[command(subcommand)]
    pub sub_command: CliSubCommand,
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