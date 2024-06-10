use std::collections::VecDeque;
use std::time::Instant;

use num_traits::ToPrimitive;
use sysinfo::{Components, Networks, System};
use tracing::error;

use crate::collect::data_point::DataPoint;
use crate::config::collector_config::{CollectorConfig, Evaluate};
use crate::ext::destructure_ext::DestructureTupleExt;

#[derive(Debug)]
pub struct Collector {
    config: CollectorConfig,
    components: Components,
    system: System,
    battery_manager: battery::Manager,

    data_points: VecDeque<DataPoint>,
    networks: Networks,
}

#[derive(Debug)]
pub struct StateRef<'a> {
    data_points: &'a VecDeque<DataPoint>,
}

impl<'a> StateRef<'a> {
    pub fn get_cpu_load(&self) -> &[u8] {
        self.data_points
            .back()
            .map(|dp| dp.cpu_load.as_slice())
            .unwrap_or(&[])
    }

    pub fn get_mem_usage(&self) -> u8 {
        self.data_points.back().map(|dp| dp.mem_usage).unwrap_or(0)
    }

    pub fn get_temp(&self) -> u8 {
        self.data_points
            .back()
            .and_then(|dp| dp.avg_temp)
            .unwrap_or(0)
    }

    pub fn get_battery_level(&self) -> u8 {
        self.data_points
            .back()
            .and_then(|dp| dp.battery_level)
            .unwrap_or(0)
    }

    pub fn get_network_speeds(&self) -> Vec<(u64, u64)> {
        self.compute_speed(self.data_points.iter().map(|dp| {
            (
                dp.ts,
                dp.network_rx_bytes.unwrap_or(0).to_f64().unwrap_or(0f64),
                dp.network_tx_bytes.unwrap_or(0).to_f64().unwrap_or(0f64),
            )
        }))
    }

    pub fn get_disk_speeds(&self) -> Vec<(u64, u64)> {
        self.compute_speed(self.data_points.iter().map(|dp| {
            (
                dp.ts,
                dp.disk_io_reads.unwrap_or(0).to_f64().unwrap_or(0f64),
                dp.disk_io_writes.unwrap_or(0).to_f64().unwrap_or(0f64),
            )
        }))
    }

    fn compute_speed(
        &self,
        mut triples: impl Iterator<Item = (Instant, f64, f64)>,
    ) -> Vec<(u64, u64)> {
        let mut speeds = Vec::new();

        let (mut prev_ts, mut prev_rx, mut prev_tx) = if let Some((ts, rx, tx)) = triples.next() {
            (ts, rx, tx)
        } else {
            return speeds;
        };

        for (ts, rx, tx) in triples {
            let elapsed = ts.duration_since(prev_ts).as_secs_f64();
            let rx_speed = ((rx - prev_rx).abs() / elapsed) as u64;
            let tx_speed = ((tx - prev_tx).abs() / elapsed) as u64;

            speeds.push((rx_speed, tx_speed));
            prev_rx = rx;
            prev_tx = tx;
            prev_ts = ts;
        }
        speeds
    }
}

impl Collector {
    pub fn new(config: CollectorConfig) -> anyhow::Result<Self> {
        let components = Components::new_with_refreshed_list();
        let system = System::new_all();
        let battery = battery::Manager::new()?;
        let networks = Networks::new_with_refreshed_list();

        Ok(Collector {
            components,
            system,
            networks,
            battery_manager: battery,
            data_points: Default::default(),
            config,
        })
    }

    pub fn update(&mut self) {
        let data_point = self.collect_all();
        self.data_points.push_back(data_point);
        if self.data_points.len() > self.config.max_history_samples {
            self.data_points.pop_front();
        }
    }

    fn collect_all(&mut self) -> DataPoint {
        let avg_temp = self.collect_cpu_temp();
        let disk_io = self.collect_disk_io_rw();
        let (disk_reads, disk_writes) = disk_io
            .map_err(|err| {
                error!(?err, "Failed to collect disk io");
                err
            })
            .destructure();
        let cpu_load = self.collect_cpu_load();
        let mem_usage = self.collect_mem_usage_percent();
        let battery_level = self
            .collect_battery_level()
            .map_err(|err| {
                error!(?err, "Failed to collect battery level");
                err
            })
            .unwrap_or(None);

        let (network_rx_bytes, network_tx_bytes) = self.collect_network_rx_tx_bytes().destructure();

        DataPoint {
            ts: Instant::now(),
            avg_temp,
            disk_io_reads: disk_reads,
            disk_io_writes: disk_writes,
            cpu_load,
            mem_usage,
            battery_level,
            network_rx_bytes,
            network_tx_bytes,
        }
    }

    fn collect_disk_io_rw(&mut self) -> anyhow::Result<Option<(u64, u64)>> {
        let mut count = 9;
        let mut total_reads = 0f64;
        let mut total_writes = 0f64;

        procfs::diskstats()?
            .into_iter()
            .filter(|disk| {
                self.config
                    .disks_names
                    .iter()
                    .any(|disk_filter| disk_filter.evaluate(disk))
            })
            .for_each(|disk| {
                total_reads += disk.reads as f64;
                total_writes += disk.writes as f64;
                count += 1;
            });

        if count == 0 {
            return Ok(None);
        }

        Ok(Some((
            (total_reads / count as f64) as u64,
            (total_writes / count as f64) as u64,
        )))
    }

    fn collect_network_rx_tx_bytes(&mut self) -> Option<(u64, u64)> {
        self.networks.refresh_list();

        let mut count = 0;
        let mut total_rx = 0f64;
        let mut total_tx = 0f64;

        self.networks
            .iter()
            .filter(|(name, network_data)| {
                self.config
                    .network_interfaces
                    .iter()
                    .any(|iface| iface.evaluate(&(name, *network_data)))
            })
            .for_each(|(_, network_data)| {
                total_rx += network_data.total_received() as f64;
                total_tx += network_data.total_transmitted() as f64;
                count += 1;
            });

        if count == 0 {
            return None;
        }

        Some((
            (total_rx / count as f64) as u64,
            (total_tx / count as f64) as u64,
        ))
    }

    fn collect_mem_usage_percent(&mut self) -> u8 {
        self.system.refresh_memory();
        (self.system.used_memory() as f32 / self.system.total_memory() as f32 * 100.0) as u8
    }
    fn collect_battery_level(&mut self) -> anyhow::Result<Option<u8>> {
        if let Some(battery) = self.battery_manager.batteries()?.next() {
            let mut battery = battery?;
            self.battery_manager.refresh(&mut battery)?;
            return Ok(Some(
                battery
                    .state_of_charge()
                    .get::<battery::units::ratio::percent>() as u8,
            ));
        }

        Ok(None)
    }

    fn collect_cpu_temp(&mut self) -> Option<u8> {
        self.components.refresh_list();

        let mut temp_total = 0f32;
        let mut count = 0;

        self.components
            .iter()
            .filter(|component| {
                self.config
                    .temperatures
                    .iter()
                    .any(|predicate| predicate.evaluate(component.label()))
            })
            .for_each(|component| {
                temp_total += component.temperature();
                count += 1;
            });

        if count == 0 {
            return None;
        }

        Some((temp_total / count as f32) as u8)
    }

    fn collect_cpu_load(&mut self) -> Vec<u8> {
        self.system.refresh_cpu();
        self.system
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage() as u8)
            .collect()
    }
    pub fn get_state(&self) -> StateRef {
        StateRef {
            data_points: &self.data_points,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::collector_config::{DiskFilter, NetworkFilter, Predicate};

    use super::*;

    #[test]
    fn test_collector() {
        let config = CollectorConfig {
            max_history_samples: 9,
            sample_interval: Default::default(),
            disks_names: vec![DiskFilter::Name(Predicate::Equal("nvme0n1".to_string()))],
            network_interfaces: vec![NetworkFilter::Name(Predicate::Equal("wlp1s0".to_string()))],
            temperatures: vec![Predicate::StartsWith("k10temp".to_string())],
        };

        let collector = Collector::new(config);
        assert!(collector.is_ok());
        let mut collector = collector.unwrap();

        let data_point = collector.collect_all();
        assert!(data_point.avg_temp.is_some());
        assert!(data_point.disk_io_reads.is_some());
        assert!(data_point.disk_io_writes.is_some());
        assert!(!data_point.cpu_load.is_empty());
        assert!(data_point.mem_usage > 0);
        assert!(data_point.battery_level.is_some());
        assert!(data_point.network_rx_bytes.is_some());
        assert!(data_point.network_tx_bytes.is_some());
    }
}
