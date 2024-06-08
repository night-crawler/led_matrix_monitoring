use procfs::DiskStat;
use serde::{Deserialize, Serialize};
use sysinfo::NetworkData;


pub trait Evaluate<T> where T: ?Sized {
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
            DiskFilter::MajorMinor(major, minor) => {
                value.major == *major && value.minor == *minor
            }
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


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CollectorConfig {
    pub max_history_samples: usize,
    #[serde(with = "humantime_serde", default = "super::default_sample_interval")]
    pub sample_interval: std::time::Duration,
    pub disks_names: Vec<DiskFilter>,
    pub network_interfaces: Vec<NetworkFilter>,
    
    pub temperatures: Vec<Predicate>,
}

