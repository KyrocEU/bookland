use serde_json::Value;
use std::process::Command;

#[derive(Clone, Default)]
pub struct AirPodsStatus {
    pub name: Option<String>,
    pub left: Option<u8>,
    pub right: Option<u8>,
    pub case: Option<u8>,
    pub overall: Option<u8>,
    pub error: Option<String>,
}

impl AirPodsStatus {
    pub fn compact_battery(&self) -> Option<String> {
        match (self.left, self.right, self.overall, self.case) {
            (Some(left), Some(right), _, _) => Some(format!("L{left}% R{right}%")),
            (Some(left), None, _, _) => Some(format!("L{left}%")),
            (None, Some(right), _, _) => Some(format!("R{right}%")),
            (_, _, Some(overall), _) => Some(format!("{overall}%")),
            (_, _, _, Some(case)) => Some(format!("Case {case}%")),
            _ => None,
        }
    }

    pub fn compact_label(&self) -> Option<String> {
        let name = self.name.as_ref()?;
        let mut label: String = name.chars().take(18).collect();
        if name.chars().count() > 18 { label.push('…'); }
        if let Some(battery) = self.compact_battery() {
            label.push_str(" · ");
            label.push_str(&battery);
        }
        Some(label)
    }
}

fn percentage(value: &Value) -> Option<u8> {
    let number = value.as_u64().or_else(|| value.as_str()
        .and_then(|text| text.trim().trim_end_matches('%').parse::<u64>().ok()))?;
    (number <= 100).then_some(number as u8)
}

fn battery(info: &Value, part: &str) -> Option<u8> {
    info.as_object()?.iter().find_map(|(key, value)| {
        let key = key.to_ascii_lowercase();
        (key.contains("battery") && key.contains(part))
            .then(|| percentage(value)).flatten()
    })
}

fn overall_battery(info: &Value) -> Option<u8> {
    info.as_object()?.iter().find_map(|(key, value)| {
        let key = key.to_ascii_lowercase();
        (key.contains("battery") && !["left", "right", "case"]
            .iter().any(|part| key.contains(part)))
            .then(|| percentage(value)).flatten()
    })
}

fn connected_airpods(root: &Value) -> AirPodsStatus {
    let Some(controller) = root["SPBluetoothDataType"].as_array()
        .and_then(|rows| rows.first()) else { return AirPodsStatus::default(); };
    let Some(fields) = controller.as_object() else { return AirPodsStatus::default(); };
    for (section, devices) in fields {
        let section = section.to_ascii_lowercase();
        if !section.contains("connected") || section.contains("not_connected") { continue; }
        let Some(devices) = devices.as_array() else { continue; };
        for device in devices {
            let Some(named) = device.as_object() else { continue; };
            for (name, info) in named {
                if !name.to_ascii_lowercase().contains("airpods") { continue; }
                return AirPodsStatus {
                    name: Some(name.clone()),
                    left: battery(info, "left"),
                    right: battery(info, "right"),
                    case: battery(info, "case"),
                    overall: overall_battery(info),
                    error: None,
                };
            }
        }
    }
    AirPodsStatus::default()
}

pub fn fetch() -> AirPodsStatus {
    let output = Command::new("/usr/sbin/system_profiler")
        .args(["SPBluetoothDataType", "-json"]).output();
    match output {
        Ok(output) if output.status.success() => {
            serde_json::from_slice(&output.stdout).map(|value| connected_airpods(&value))
                .unwrap_or_else(|_| AirPodsStatus { error: Some("Bluetooth data unavailable".into()), ..Default::default() })
        }
        _ => AirPodsStatus { error: Some("Bluetooth data unavailable".into()), ..Default::default() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_connected_batteries_without_paired_devices() {
        let record: Value = serde_json::from_str(r#"{"SPBluetoothDataType":[{"device_not_connected":[{"AirPods":{"device_batteryLevel":"98%"}}],"devices_connected":[{"AirPods Pro":{"device_batteryLevelLeft":"63%","device_batteryLevelRight":"58%","device_batteryLevelCase":"91%"}}]}]}"#).unwrap();
        let status = connected_airpods(&record);
        assert_eq!(status.name.as_deref(), Some("AirPods Pro"));
        assert_eq!((status.left, status.right, status.case), (Some(63), Some(58), Some(91)));
        assert_eq!(status.compact_battery().as_deref(), Some("L63% R58%"));
        assert_eq!(status.compact_label().as_deref(), Some("AirPods Pro · L63% R58%"));
    }
    #[test]
    fn compact_status_requires_a_connected_name() {
        assert!(AirPodsStatus::default().compact_label().is_none());
        let status = AirPodsStatus { name: Some("AirPods Max".into()), overall: Some(72),
            ..Default::default() };
        assert_eq!(status.compact_label().as_deref(), Some("AirPods Max · 72%"));
    }
}
