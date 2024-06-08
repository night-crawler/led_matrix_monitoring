use crate::config::collector_config::{CollectorConfig, Evaluate};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::time::Instant;
use sysinfo::{Components, Networks, System};
use tracing::error;

#[derive(Debug)]
pub struct DataPoint {
    pub ts: Instant,
    pub avg_temp: Option<u8>,
    pub disk_io_reads: Option<u64>,
    pub disk_io_writes: Option<u64>,
    pub cpu_load: Vec<u8>,
    pub mem_usage: u8,
    pub battery_level: Option<u8>,
    pub network_rx_bytes: Option<u64>,
    pub network_tx_bytes: Option<u64>,
}


#[derive(Debug)]
pub struct Collector {
    config: CollectorConfig,
    components: Components,
    system: System,
    battery_manager: battery::Manager,

    data_points: VecDeque<DataPoint>,
    networks: Networks,
}

trait DestructureTupleExt<A, B> {
    fn destructure(self) -> (Option<A>, Option<B>);
}

impl<A, B> DestructureTupleExt<A, B> for Option<(A, B)> {
    fn destructure(self) -> (Option<A>, Option<B>) {
        self.map(|(a, b)| (Some(a), Some(b)))
            .unwrap_or((None, None))
    }
}

impl<A, B, E> DestructureTupleExt<A, B> for Result<Option<(A, B)>, E>
where
    E: Debug,
{
    fn destructure(self) -> (Option<A>, Option<B>) {
        match self {
            Ok(Some((a, b))) => (Some(a), Some(b)),
            Ok(None) => (None, None),
            Err(_) => (None, None),
        }
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
    
    pub fn last_data_point(&self) -> Option<&DataPoint> {
        self.data_points.back()
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

    pub fn compute_network_rx_tx_speed(&self) -> (u64, u64) {
        let mut rx_speed = 0;
        let mut tx_speed = 0;
        let last = self.data_points.back();
        let before_last = self.data_points.iter().rev().nth(1);

        if let (
            Some(DataPoint {
                ts: ts0,
                network_rx_bytes: Some(rx_0),
                network_tx_bytes: Some(tx_0),
                ..
            }),
            Some(DataPoint {
                ts: ts1,
                network_rx_bytes: Some(rx_1),
                network_tx_bytes: Some(tx_1),
                ..
            })
        ) = (before_last, last)
        {
            let elapsed = ts1.duration_since(*ts0).as_secs_f64();
            rx_speed = ((rx_1 - rx_0) as f64 / elapsed) as u64;
            tx_speed = ((tx_1 - tx_0) as f64 / elapsed) as u64;
        }

        (rx_speed, tx_speed)
    }
    
    pub fn compute_disk_io_speed(&self) -> (u64, u64) {
        let mut read_speed = 0;
        let mut write_speed = 0;
        let last = self.data_points.back();
        let before_last = self.data_points.iter().rev().nth(1);

        if let (
            Some(DataPoint {
                ts: ts0,
                disk_io_reads: Some(read_0),
                disk_io_writes: Some(write_0),
                ..
            }),
            Some(DataPoint {
                ts: ts1,
                disk_io_reads: Some(read_1),
                disk_io_writes: Some(write_1),
                ..
            })
        ) = (before_last, last)
        {
            let elapsed = ts1.duration_since(*ts0).as_secs_f64();
            read_speed = ((read_1 - read_0) as f64 / elapsed) as u64;
            write_speed = ((write_1 - write_0) as f64 / elapsed) as u64;
        }

        (read_speed, write_speed)
    }
    
    pub fn get_cpu_load(&self) -> Option<&[u8]> {
        self.data_points.back().map(|dp| dp.cpu_load.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::collector_config::{DiskFilter, NetworkFilter, Predicate};

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
