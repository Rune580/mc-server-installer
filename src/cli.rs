use clap::{command, Subcommand, Parser};

#[derive(Parser, Clone, Debug)]
pub struct Cli {
    #[command(subcommand)]
    pub modpack: ModPack,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ModPack {
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
}