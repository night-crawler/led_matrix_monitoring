use crate::collect::data_point::DataPoint;
use std::collections::VecDeque;
use std::time::Instant;
use num_traits::ToPrimitive;

#[derive(Debug)]
pub struct SensorState<'a> {
    pub data_points: &'a VecDeque<DataPoint>,
}

impl<'a> SensorState<'a> {
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
