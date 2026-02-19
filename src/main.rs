mod app;
mod caddy;
mod compose;
mod docker;
mod model;
mod ui;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "lcp", version, about = "Local Caddy Proxy Manager")]
struct Cli {}

#[tokio::main]
async fn main() -> Result<()> {
    let _cli = Cli::parse();

    let mut app = app::App::new().await?;
    app.run().await?;

    Ok(())
}
