extern crate core;

use clap::Parser;

use crate::api::uds::RenderRequest;
use crate::cli::CmdArgs;
use crate::collect::collector::Collector;
use crate::config::collector_config::Config;
use crate::init::init_tracing;
use crate::render::renderer::Renderer;

mod api;
mod cli;
mod collect;
mod config;
mod constants;
mod ext;
mod init;
mod render;

fn main() -> anyhow::Result<()> {
    init_tracing()?;

    let cmd_args = CmdArgs::parse();
    let config: Config = toml::from_str(&std::fs::read_to_string(cmd_args.config)?)?;
    let delay = config.collector.sample_interval;

    let uds = api::uds::UdsClient::new(&config.socket)?;
    let mut collector = Collector::new(config.collector)?;
    let mut max_brightness = config.render.max_brightness.unwrap_or(255);
    loop {
        if let Some(file) = config.render.max_brightness_file.as_ref() {
            max_brightness = std::fs::read_to_string(file)?.trim().parse()?;
        }

        collector.update();
        let mut left_renderer = Renderer::new(max_brightness);
        for render_type in config.render.left.iter() {
            left_renderer.render(render_type, collector.get_state())?;
        }

        let mut right_renderer = Renderer::new(max_brightness);
        for render_type in config.render.right.iter() {
            right_renderer.render(render_type, collector.get_state())?;
        }

        let left_data = left_renderer.save_to_in_memory_png()?;
        let right_data = right_renderer.save_to_in_memory_png()?;

        uds.send_request(RenderRequest {
            left_image: Some(&left_data),
            right_image: Some(&right_data),
        })?;

        std::thread::sleep(delay);
    }
}
