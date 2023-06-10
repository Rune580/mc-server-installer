use clap::{command, Subcommand, Parser};

#[derive(Parser, Clone)]
pub struct Cli {
    #[command(subcommand)]
    pub modpack: ModPack,
}

#[derive(Clone, Subcommand)]
pub enum ModPack {
    Flame {
        #[clap(env, long)]
        api_key: String,
        #[clap(env, long)]
        project_id: u64,
        #[clap(env, long)]
        version: String,
    },
}