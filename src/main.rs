extern crate core;

use crate::api::uds::RenderRequest;
use crate::collect::collector::Collector;
use crate::config::collector_config::Config;
use crate::init::init_tracing;
use crate::render::renderer::Renderer;

mod api;
mod collect;
mod config;
mod constants;
mod ext;
mod init;
mod render;

fn main() -> anyhow::Result<()> {
    init_tracing()?;

    let config: Config = toml::from_str(&std::fs::read_to_string("./example_config.toml")?)?;
    let delay = config.collector.sample_interval;

    let uds = api::uds::UdsClient::new(&config.socket)?;
    let mut collector = Collector::new(config.collector)?;

    loop {
        collector.update();
        let mut left_renderer = Renderer::new();
        for render_type in config.render.left.iter() {
            left_renderer.render(render_type, collector.get_state())?;
        }

        let mut right_renderer = Renderer::new();
        for render_type in config.render.right.iter() {
            right_renderer.render(render_type, collector.get_state())?;
        }

        // left_renderer.render_cpu(10, 10, collector.get_cpu_load(), 1.0)?;
        // left_renderer.render_average_cpu(7, 20, 9, collector.get_cpu_load(), 1.0)?;
        // left_renderer.plot_io(27, 7, collector.get_network_speeds().into_iter(), 6.0)?;
        //
        // right_renderer.plot_io(27, 7, collector.get_disk_speeds().into_iter(), 6.0)?;
        // right_renderer.render_horizontal_bar(collector.get_mem_usage() as u64, 100, 19, 0, 9, 3.0)?;
        // right_renderer.render_horizontal_bar(collector.get_mem_usage() as u64, 100, 20, 0, 9, 3.0)?;
        //
        // right_renderer.render_horizontal_bar(collector.get_temp() as u64, 100, 16, 0, 9, 3.0)?;
        // right_renderer.render_horizontal_bar(collector.get_temp() as u64, 100, 17, 0, 9, 3.0)?;
        //
        // right_renderer.render_battery(0, 10,  collector.get_battery_level())?;

        let left_data = left_renderer.save_to_in_memory_png()?;
        let right_data = right_renderer.save_to_in_memory_png()?;

        uds.send_request(RenderRequest {
            left_image: Some(&left_data),
            right_image: Some(&right_data),
        })?;

        std::thread::sleep(delay);
    }
}
