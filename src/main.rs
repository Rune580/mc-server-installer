use std::str::FromStr;
use clap::Parser;
use dotenv::dotenv;
use simplelog::{ColorChoice, CombinedLogger, TerminalMode, TermLogger, WriteLogger};
use cli::Cli;
use crate::fs_utils::{ensure_dir, get_log_file};
use crate::modloader::fabric::install_fabric;
use crate::modloader::forge::install_forge;
use crate::modpack::ftb::IdOrSearch;
use crate::version::McVersion;

mod cli;
mod modpack;
mod modloader;
mod version;
pub mod fs_utils;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    dotenv().ok();

    let cli = Cli::parse();

    CombinedLogger::init(
        vec![
            TermLogger::new(cli.rust_log.into(), simplelog::Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(cli.rust_log.into(), simplelog::Config::default(), get_log_file().unwrap()),
        ]
    )?;

    match cli.sub_command {
        cli::CliSubCommand::Flame {
            api_key,
            project_id,
            version,
            target_dir,
        } => modpack::flame::handle_flame(api_key, project_id, version, target_dir).await?,
        cli::CliSubCommand::Ftb {
            search_terms,
            mc_version,
            id,
            version,
            target_dir,
        } => {
            let args = if id.is_some() {
                IdOrSearch::Id(id.unwrap())
            } else {
                IdOrSearch::Search {
                    terms: search_terms.unwrap(),
                    mc_version,
                }
            };

            modpack::ftb::handle_ftb(args, version, target_dir)
                .await?;
        }
        cli::CliSubCommand::Forge {
            mc_version,
            version,
            target_dir,
        } => {
            let mc_version = McVersion::from_str(&mc_version)?;
            ensure_dir(&target_dir)?;
            install_forge(mc_version, &version, &target_dir)
                .await?;
        }
        cli::CliSubCommand::Fabric {
            mc_version,
            version,
            target_dir,
        } => {
            let mc_version = McVersion::from_str(&mc_version)?;
            ensure_dir(&target_dir)?;
            install_fabric(mc_version, &version, &target_dir)
                .await?;
        }
    }

    Ok(())
}
