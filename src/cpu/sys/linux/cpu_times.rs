use std::fs;
use std::io;
use std::str::FromStr;
use std::time::Duration;

use crate::cpu::os::{linux::CpuTimesExt as _, unix::CpuTimesExt as _};
use crate::utils::invalid_data;
use crate::{Count, TICKS_PER_SECOND};

/// Every attribute represents the seconds the CPU has spent in the given mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuTimes {
    pub(crate) user: Duration,
    pub(crate) nice: Duration,
    pub(crate) system: Duration,
    pub(crate) idle: Duration,
    pub(crate) iowait: Duration,
    pub(crate) irq: Duration,
    pub(crate) softirq: Duration,
    pub(crate) steal: Duration,
    pub(crate) guest: Duration,
    pub(crate) guest_nice: Duration,
}

impl CpuTimes {
    /// Time spent by normal processes executing in user mode;
    /// on Linux this also includes guest time.
    pub fn user(&self) -> Duration {
        self.user
    }

    /// Time spent by processes executing in kernel mode.
    pub fn system(&self) -> Duration {
        self.system
    }

    /// Time spent doing nothing.
    pub fn idle(&self) -> Duration {
        self.idle
    }

    /// New method, not in Python psutil.
    pub fn busy(&self) -> Duration {
        // TODO: what about guest and guest_nice?
        self.user()
            + self.system()
            + self.nice()
            + self.iowait() // TODO: is iowait idle time?
            + self.irq()
            + self.softirq()
            + self.steal()
    }

    /// New method, not in Python psutil.
    pub fn total(&self) -> Duration {
        self.busy() + self.idle()
    }
}

impl FromStr for CpuTimes {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fields = s
            .split_whitespace()
            .skip(1)
            .map(|entry| Ok(try_parse!(entry, Count::from_str)))
            .collect::<io::Result<Vec<Count>>>()?
            .into_iter()
            .map(|entry| Duration::from_secs_f64(entry as f64 / *TICKS_PER_SECOND))
            .collect::<Vec<Duration>>();

        if fields.len() != 10 {
            return Err(invalid_data(&format!(
                "Expected 10 fields but got {}",
                fields.len()
            )));
        }

        let user = fields[0];
        let nice = fields[1];
        let system = fields[2];
        let idle = fields[3];
        let iowait = fields[4];
        let irq = fields[5];
        let softirq = fields[6];
        let steal = fields[7];
        let guest = fields[8];
        let guest_nice = fields[9];

        Ok(CpuTimes {
            user,
            nice,
            system,
            idle,
            iowait,
            irq,
            softirq,
            steal,
            guest,
            guest_nice,
        })
    }
}

pub fn cpu_times() -> io::Result<CpuTimes> {
    let data = fs::read_to_string("/proc/stat")?;
    let lines = data.lines().collect::<Vec<&str>>();

    if lines.is_empty() {
        return Err(invalid_data("'/proc/stat' is empty"));
    }

    let line = lines[0];

    CpuTimes::from_str(&line)
}

pub fn cpu_times_percpu() -> io::Result<Vec<CpuTimes>> {
    let data = fs::read_to_string("/proc/stat")?;
    let lines = data
        .lines()
        .skip(1)
        .take_while(|line| line.starts_with("cpu"))
        .collect::<Vec<&str>>();

    if lines.is_empty() {
        return Err(invalid_data("'/proc/stat' is missing per cpu times"));
    }

    let mut cpu_times = Vec::new();
    for line in lines {
        cpu_times.push(CpuTimes::from_str(&line)?);
    }

    Ok(cpu_times)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu_times() {
        let line = "cpu  11867200 6935 2978038 19104017 85955 502109 144021 0 0 0";
        let result = CpuTimes::from_str(line).unwrap();
        let expected = CpuTimes {
            user: Duration::from_secs_f64(11_867_200_f64 / *TICKS_PER_SECOND),
            nice: Duration::from_secs_f64(6935_f64 / *TICKS_PER_SECOND),
            system: Duration::from_secs_f64(2_978_038_f64 / *TICKS_PER_SECOND),
            idle: Duration::from_secs_f64(19_104_017_f64 / *TICKS_PER_SECOND),
            iowait: Duration::from_secs_f64(85955_f64 / *TICKS_PER_SECOND),
            irq: Duration::from_secs_f64(502_109_f64 / *TICKS_PER_SECOND),
            softirq: Duration::from_secs_f64(144_021_f64 / *TICKS_PER_SECOND),
            steal: Duration::default(),
            guest: Duration::default(),
            guest_nice: Duration::default(),
        };
        assert_eq!(result, expected);
    }
}
