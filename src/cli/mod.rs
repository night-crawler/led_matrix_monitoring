use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = r###"led_matrix_monitoring"###)]
pub struct CmdArgs {
    /// Path to the configuration file.
    #[arg(short, long, default_value = "/etc/led_matrix/monitoring.toml")]
    pub config: PathBuf,
}
