use clap::Parser;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, TerminalMode, TermLogger};
use cli::Cli;

mod cli;
mod modpack;
mod modloader;
mod version;
pub mod fs_utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Info, simplelog::Config::default(), TerminalMode::Mixed, ColorChoice::Auto)
        ]
    )?;

    match cli.modpack {
        cli::ModPack::Flame {
            api_key,
            project_id,
            version,
        } => modpack::flame::handle_flame(api_key, project_id, version).await?,
    }

    Ok(())
}
