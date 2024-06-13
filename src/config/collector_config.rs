use procfs::DiskStat;
use serde::{Deserialize, Serialize};
use sysinfo::NetworkData;

pub trait Evaluate<T>
where
    T: ?Sized,
{
    fn evaluate(&self, value: &T) -> bool;
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Predicate {
    Contains(String),
    StartsWith(String),
    EndsWith(String),
    Equal(String),
    IEqual(String),
}

impl Evaluate<str> for Predicate {
    fn evaluate(&self, value: &str) -> bool {
        match self {
            Predicate::Contains(pattern) => value.contains(pattern),
            Predicate::StartsWith(pattern) => value.starts_with(pattern),
            Predicate::EndsWith(pattern) => value.ends_with(pattern),
            Predicate::Equal(pattern) => value == pattern,
            Predicate::IEqual(pattern) => value.eq_ignore_ascii_case(pattern),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum DiskFilter {
    Name(Predicate),
    MajorMinor(i32, i32),
}

impl Evaluate<DiskStat> for DiskFilter {
    fn evaluate(&self, value: &DiskStat) -> bool {
        match self {
            DiskFilter::Name(predicate) => predicate.evaluate(&value.name),
            DiskFilter::MajorMinor(major, minor) => value.major == *major && value.minor == *minor,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum NetworkFilter {
    Name(Predicate),
    MacAddress(Predicate),
}

impl Evaluate<(&String, &NetworkData)> for NetworkFilter {
    fn evaluate(&self, (name, network_data): &(&String, &NetworkData)) -> bool {
        match self {
            NetworkFilter::Name(predicate) => predicate.evaluate(name),
            NetworkFilter::MacAddress(predicate) => {
                predicate.evaluate(&network_data.mac_address().to_string())
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum RenderType {
    Cpu {
        mid_point: u8,
        max_height: u8,
        k: f32,
    },
    AverageCpu {
        start_x: u8,
        start_y: u8,
        end_y: u8,
        k: f32,
    },
    Network {
        mid_point: u8,
        max_height: u8,
        k: f32,
    },
    Disk {
        mid_point: u8,
        max_height: u8,
        k: f32,
    },
    Mem {
        max_value: u8,
        start_y: u8,
        start_x: u8,
        end_x: u8,
        k: f32,
    },
    Temp {
        max_value: u8,
        start_y: u8,
        start_x: u8,
        end_x: u8,
        k: f32,
    },
    Battery {
        start_y: u8,
        max_height: u8,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectorConfig {
    pub max_history_samples: usize,
    #[serde(with = "humantime_serde", default = "super::default_sample_interval")]
    pub sample_interval: std::time::Duration,
    pub disk_names: Vec<DiskFilter>,
    pub network_interfaces: Vec<NetworkFilter>,

    pub temperatures: Vec<Predicate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenderConfig {
    #[serde(default)]
    pub left: Vec<RenderType>,

    #[serde(default)]
    pub right: Vec<RenderType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub socket: String,
    pub collector: CollectorConfig,
    pub render: RenderConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_sample_config() {
        let collector_config = CollectorConfig {
            max_history_samples: 10,
            sample_interval: std::time::Duration::from_millis(170),
            disk_names: vec![DiskFilter::Name(Predicate::Equal("nvme0n1".to_string()))],
            network_interfaces: vec![NetworkFilter::Name(Predicate::Equal("wlp1s0".to_string()))],
            temperatures: vec![Predicate::StartsWith("k10temp".to_string())],
        };

        let render_config = RenderConfig {
            left: vec![
                RenderType::Cpu {
                    mid_point: 10,
                    max_height: 10,
                    k: 1.0,
                },
                RenderType::AverageCpu {
                    start_x: 7,
                    start_y: 20,
                    end_y: 9,
                    k: 1.0,
                },
                RenderType::Network {
                    mid_point: 27,
                    max_height: 7,
                    k: 6.0,
                },
            ],

            right: vec![
                RenderType::Disk {
                    mid_point: 27,
                    max_height: 7,
                    k: 6.0,
                },
                RenderType::Mem {
                    max_value: 100,
                    start_y: 19,
                    start_x: 0,
                    end_x: 9,
                    k: 3.0,
                },
                RenderType::Mem {
                    max_value: 100,
                    start_y: 20,
                    start_x: 0,
                    end_x: 9,
                    k: 3.0,
                },
                RenderType::Temp {
                    max_value: 100,
                    start_y: 16,
                    start_x: 0,
                    end_x: 9,
                    k: 3.0,
                },
                RenderType::Temp {
                    max_value: 100,
                    start_y: 17,
                    start_x: 0,
                    end_x: 9,
                    k: 3.0,
                },
                RenderType::Battery {
                    start_y: 0,
                    max_height: 10,
                },
            ],
        };

        let config = Config {
            socket: "/tmp/led-matrix.sock".to_string(),
            collector: collector_config,
            render: render_config,
        };

        let value = toml::ser::to_string(&config).unwrap();
        std::fs::write("/tmp//example_config.toml", value).unwrap();
    }
}
