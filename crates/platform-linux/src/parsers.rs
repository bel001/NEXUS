use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct CpuTextInfo {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub logical_count: Option<u32>,
}

pub(crate) fn parse_os_release(input: &str) -> BTreeMap<String, String> {
    input
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            let value = value.trim();
            let unquoted = value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .or_else(|| {
                    value
                        .strip_prefix('\'')
                        .and_then(|value| value.strip_suffix('\''))
                })
                .unwrap_or(value);
            Some((key.trim().to_owned(), unquoted.to_owned()))
        })
        .collect()
}

pub(crate) fn parse_mem_total_bytes(input: &str) -> Result<u64, &'static str> {
    let line = input
        .lines()
        .find(|line| line.trim_start().starts_with("MemTotal:"))
        .ok_or("MemTotal is missing")?;
    let mut fields = line.split_whitespace();
    if fields.next() != Some("MemTotal:") {
        return Err("MemTotal has an invalid label");
    }
    let kibibytes = fields
        .next()
        .ok_or("MemTotal has no value")?
        .parse::<u64>()
        .map_err(|_| "MemTotal is not numeric")?;
    if fields.next() != Some("kB") {
        return Err("MemTotal does not use kB");
    }
    kibibytes
        .checked_mul(1024)
        .ok_or("MemTotal overflows bytes")
}

pub(crate) fn parse_cpuinfo(input: &str) -> CpuTextInfo {
    let mut manufacturer = None;
    let mut model = None;
    let mut logical = BTreeSet::new();

    for line in input.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "vendor_id" | "CPU implementer" if manufacturer.is_none() && !value.is_empty() => {
                manufacturer = Some(value.to_owned());
            }
            "model name" | "Processor" | "Hardware" if model.is_none() && !value.is_empty() => {
                model = Some(value.to_owned());
            }
            "processor" => {
                if let Ok(index) = value.parse::<u32>() {
                    logical.insert(index);
                }
            }
            _ => {}
        }
    }

    CpuTextInfo {
        manufacturer,
        model,
        logical_count: u32::try_from(logical.len()).ok().filter(|count| *count > 0),
    }
}

pub(crate) fn parse_cpu_list(input: &str) -> Result<BTreeSet<u32>, &'static str> {
    let mut result = BTreeSet::new();
    for part in input.trim().split(',').filter(|part| !part.is_empty()) {
        if let Some((start, end)) = part.split_once('-') {
            let start = start
                .trim()
                .parse::<u32>()
                .map_err(|_| "invalid CPU range start")?;
            let end = end
                .trim()
                .parse::<u32>()
                .map_err(|_| "invalid CPU range end")?;
            if start > end {
                return Err("reversed CPU range");
            }
            result.extend(start..=end);
        } else {
            result.insert(
                part.trim()
                    .parse::<u32>()
                    .map_err(|_| "invalid CPU index")?,
            );
        }
    }
    if result.is_empty() {
        Err("CPU list is empty")
    } else {
        Ok(result)
    }
}

pub(crate) struct TopologyCounts {
    pub core_count: Option<u32>,
    pub package_count: Option<u32>,
    pub has_problem: bool,
}

pub(crate) fn topology_counts(cpu_root: &Path, online: &BTreeSet<u32>) -> TopologyCounts {
    let mut core_groups_by_cpu = BTreeMap::<u32, BTreeSet<u32>>::new();
    let mut packages = BTreeSet::<i32>::new();
    let mut package_by_cpu = BTreeMap::<u32, i32>::new();
    let mut complete_cores = true;
    let mut complete_packages = true;

    for cpu in online {
        let topology = cpu_root.join(format!("cpu{cpu}/topology"));
        let group_text = fs::read_to_string(topology.join("core_cpus_list"))
            .or_else(|_| fs::read_to_string(topology.join("thread_siblings_list")));
        match group_text.ok().and_then(|text| parse_cpu_list(&text).ok()) {
            Some(group) if group.contains(cpu) => {
                let online_group: BTreeSet<u32> = group.intersection(online).copied().collect();
                if online_group.is_empty() {
                    complete_cores = false;
                } else {
                    core_groups_by_cpu.insert(*cpu, online_group);
                }
            }
            _ => complete_cores = false,
        }

        match fs::read_to_string(topology.join("physical_package_id"))
            .ok()
            .and_then(|value| value.trim().parse::<i32>().ok())
        {
            Some(package) if package >= 0 => {
                packages.insert(package);
                package_by_cpu.insert(*cpu, package);
            }
            _ => complete_packages = false,
        }
    }

    if complete_cores {
        let mut covered = BTreeSet::new();
        for (cpu, group) in &core_groups_by_cpu {
            if !group.contains(cpu)
                || group
                    .iter()
                    .any(|member| core_groups_by_cpu.get(member) != Some(group))
            {
                complete_cores = false;
                break;
            }
            covered.extend(group);
        }
        complete_cores &= covered == *online;
    }

    if complete_cores && complete_packages {
        for group in core_groups_by_cpu.values() {
            let group_packages: BTreeSet<i32> = group
                .iter()
                .filter_map(|cpu| package_by_cpu.get(cpu).copied())
                .collect();
            if group_packages.len() != 1 {
                complete_cores = false;
                break;
            }
        }
    }

    let core_groups: BTreeSet<Vec<u32>> = core_groups_by_cpu
        .values()
        .map(|group| group.iter().copied().collect())
        .collect();
    let core_count = complete_cores
        .then(|| u32::try_from(core_groups.len()).ok())
        .flatten()
        .filter(|count| *count > 0);
    let package_count = complete_packages
        .then(|| u32::try_from(packages.len()).ok())
        .flatten()
        .filter(|count| *count > 0);
    TopologyCounts {
        core_count,
        package_count,
        has_problem: !complete_cores || !complete_packages,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn parses_cpu_ranges() {
        assert_eq!(
            parse_cpu_list("0-2,5").expect("valid fixture"),
            BTreeSet::from([0, 1, 2, 5])
        );
        assert!(parse_cpu_list("4-2").is_err());
    }

    #[test]
    fn parses_memory_with_checked_units() {
        assert_eq!(
            parse_mem_total_bytes("MemTotal:       1024 kB\n"),
            Ok(1_048_576)
        );
        assert!(parse_mem_total_bytes("MemTotal: 3 MB").is_err());
    }

    #[test]
    fn parses_os_release_quotes() {
        let values = parse_os_release("NAME=Linux\nPRETTY_NAME=\"Mint 22\"\n");
        assert_eq!(
            values.get("PRETTY_NAME").map(String::as_str),
            Some("Mint 22")
        );
    }

    #[test]
    fn contradictory_core_groups_do_not_produce_a_count() {
        let temp = tempfile::tempdir().expect("temporary topology");
        for (cpu, group) in [(0, "0-1\n"), (1, "1\n")] {
            let topology = temp.path().join(format!("cpu{cpu}/topology"));
            fs::create_dir_all(&topology).expect("topology directory");
            fs::File::create(topology.join("core_cpus_list"))
                .and_then(|mut file| file.write_all(group.as_bytes()))
                .expect("core group");
            fs::File::create(topology.join("physical_package_id"))
                .and_then(|mut file| file.write_all(b"0\n"))
                .expect("package id");
        }

        let counts = topology_counts(temp.path(), &BTreeSet::from([0, 1]));
        assert_eq!(counts.core_count, None);
        assert_eq!(counts.package_count, Some(1));
        assert!(counts.has_problem);
    }
}
