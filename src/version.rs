use std::cmp::Ordering;

#[derive(Debug)]
struct ParsedVersion {
    core: Vec<u32>,
    prerelease: Vec<String>,
}

fn parse_version(version: &str) -> ParsedVersion {
    let normalized = version.trim().trim_start_matches(|c: char| c.eq_ignore_ascii_case(&'v'));
    let without_build_metadata = normalized.split('+').next().unwrap_or("");
    let mut parts = without_build_metadata.splitn(2, '-');
    let core_raw = parts.next().unwrap_or("");
    let prerelease_raw = parts.next();

    let mut core = Vec::new();
    for part in core_raw.split('.') {
        let numeric_part: String = part.chars().filter(|c| c.is_ascii_digit()).collect();
        if let Ok(val) = numeric_part.parse::<u32>() {
            core.push(val);
        } else {
            core.push(0);
        }
    }
    if core.is_empty() {
        core.push(0);
    }

    let mut prerelease = Vec::new();
    if let Some(pr) = prerelease_raw {
        for part in pr.split('.') {
            let t = part.trim();
            if !t.is_empty() {
                prerelease.push(t.to_string());
            }
        }
    }

    ParsedVersion { core, prerelease }
}

fn compare_prerelease_part(left: &str, right: &str) -> Ordering {
    let left_is_numeric = left.chars().all(|c| c.is_ascii_digit());
    let right_is_numeric = right.chars().all(|c| c.is_ascii_digit());

    if left_is_numeric && right_is_numeric {
        let left_val = left.parse::<u32>().unwrap_or(0);
        let right_val = right.parse::<u32>().unwrap_or(0);
        left_val.cmp(&right_val)
    } else if left_is_numeric {
        Ordering::Less
    } else if right_is_numeric {
        Ordering::Greater
    } else {
        left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase())
    }
}

fn compare_version(left: &str, right: &str) -> Ordering {
    let left_parsed = parse_version(left);
    let right_parsed = parse_version(right);

    let max_length = std::cmp::max(left_parsed.core.len(), right_parsed.core.len());
    for i in 0..max_length {
        let left_part = left_parsed.core.get(i).copied().unwrap_or(0);
        let right_part = right_parsed.core.get(i).copied().unwrap_or(0);
        match left_part.cmp(&right_part) {
            Ordering::Equal => continue,
            other => return other,
        }
    }

    let left_is_release = left_parsed.prerelease.is_empty();
    let right_is_release = right_parsed.prerelease.is_empty();

    if left_is_release && right_is_release {
        return Ordering::Equal;
    }
    if left_is_release && !right_is_release {
        return Ordering::Greater;
    }
    if !left_is_release && right_is_release {
        return Ordering::Less;
    }

    let max_pre_length = std::cmp::max(left_parsed.prerelease.len(), right_parsed.prerelease.len());
    for i in 0..max_pre_length {
        let left_part = left_parsed.prerelease.get(i);
        let right_part = right_parsed.prerelease.get(i);

        match (left_part, right_part) {
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(l), Some(r)) => {
                let cmp = compare_prerelease_part(l, r);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
        }
    }

    Ordering::Equal
}

pub fn is_boot_version_in_range(boot_version: &str, version_range: Option<&str>) -> bool {
    let range = match version_range {
        Some(r) => r.trim(),
        None => return true,
    };
    if range.is_empty() {
        return true;
    }

    if !range.contains(',') {
        return compare_version(boot_version, range) != Ordering::Less;
    }

    let is_lower_inclusive = range.starts_with('[');
    let is_upper_inclusive = range.ends_with(']');

    if !range.starts_with('[') && !range.starts_with('(') {
        return true;
    }
    if !range.ends_with(']') && !range.ends_with(')') {
        return true;
    }

    let body = &range[1..range.len() - 1];
    let bounds: Vec<&str> = body.split(',').map(|s| s.trim()).collect();
    if bounds.len() < 2 {
        return true;
    }

    let lower_bound = bounds[0];
    let upper_bound = bounds[1];

    if !lower_bound.is_empty() {
        let cmp_lower = compare_version(boot_version, lower_bound);
        if cmp_lower == Ordering::Less || (cmp_lower == Ordering::Equal && !is_lower_inclusive) {
            return false;
        }
    }

    if !upper_bound.is_empty() {
        let cmp_upper = compare_version(boot_version, upper_bound);
        if cmp_upper == Ordering::Greater || (cmp_upper == Ordering::Equal && !is_upper_inclusive) {
            return false;
        }
    }

    true
}

pub fn format_version_range(version_range: &str) -> String {
    let range = version_range.trim();
    if range.is_empty() {
        return String::new();
    }

    if !range.contains(',') {
        return format!(">= {range}");
    }

    let is_lower_inclusive = range.starts_with('[');
    let is_upper_inclusive = range.ends_with(']');

    if !range.starts_with('[') && !range.starts_with('(') {
        return format!(">= {range}"); // Fallback
    }

    let body = &range[1..range.len() - 1];
    let bounds: Vec<&str> = body.split(',').map(|s| s.trim()).collect();
    if bounds.len() < 2 {
        return range.to_string(); // Fallback
    }

    let lower_bound = bounds[0];
    let upper_bound = bounds[1];

    let mut parts = Vec::new();

    if !lower_bound.is_empty() {
        let op = if is_lower_inclusive { ">=" } else { ">" };
        parts.push(format!("{op} {lower_bound}"));
    }

    if !upper_bound.is_empty() {
        let op = if is_upper_inclusive { "<=" } else { "<" };
        parts.push(format!("{op} {upper_bound}"));
    }

    parts.join(", ")
}
