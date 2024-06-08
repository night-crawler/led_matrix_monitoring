use console_subscriber::ConsoleLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init_tracing() -> anyhow::Result<()> {
    let console_layer = ConsoleLayer::builder().with_default_env().spawn();
    let fmt_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_ansi(atty::is(atty::Stream::Stdout))
        .with_target(false);
    let filter_layer = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?;

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(console_layer)
        .init();

    Ok(())
}
