pub mod collector_config;

fn default_sample_interval() -> std::time::Duration {
    std::time::Duration::from_secs(1)
}
