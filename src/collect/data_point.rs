use std::fmt::Debug;
use std::time::Instant;

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
