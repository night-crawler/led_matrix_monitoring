extern crate core;

mod collect;
mod init;
mod config;
mod render;
mod constants;
mod api;

use std::time::Duration;
use tracing::{info, warn};
use crate::collect::data_point::Collector;
use crate::config::collector_config::{CollectorConfig, DiskFilter, NetworkFilter, Predicate};
use crate::init::init_tracing;
use crate::render::renderer::Renderer;


fn main() -> anyhow::Result<()> {
    init_tracing()?;

    let config = CollectorConfig {
        max_history_samples: 9,
        sample_interval: Duration::from_millis(200),
        disks_names: vec![DiskFilter::Name(Predicate::Equal("nvme0n1".to_string()))],
        network_interfaces: vec![NetworkFilter::Name(Predicate::Equal("wlp1s0".to_string()))],
        temperatures: vec![Predicate::StartsWith("k10temp".to_string())],
    };

    let delay = config.sample_interval;
    let mut collector = Collector::new(config)?;

    loop {
        collector.update();

        let mut renderer = Renderer::new();
        renderer.render_cpu(10, 10, collector.get_cpu_load().unwrap());
        renderer.save_to_file("./target/cpu.png")?;

        let Some(data_point) = collector.last_data_point() else {
            warn!("No data points yet");
            std::thread::sleep(delay);
            continue;
        };

        info!("Data point: {:?}", data_point);

        let speed = collector.compute_network_rx_tx_speed();
        info!("Network RX: {} TX: {}", speed.0, speed.1);

        let speed = collector.compute_disk_io_speed();
        info!("Disk IO Read: {} Write: {}", speed.0, speed.1);

        std::thread::sleep(delay);
    }
}
