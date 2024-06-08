#[derive(Debug, Default)]
pub struct SensorState {
    pub cpu_load_percent: [u8; 16],
    pub cpu_temp_degrees_avg: u8,
    pub mem_usage_percent: u8,
    pub battery_level_percent: u8,
    pub network_rx_speed_kbps: u16,
    pub network_tx_speed_kbps: u16,
    pub disk_io_writes_per_sec: u16,
    pub disk_io_reads_per_sec: u16,
}
