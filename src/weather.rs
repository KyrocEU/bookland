use serde_json::Value;
use std::process::Command;

#[derive(Clone, Default)]
pub struct Weather {
    pub place: String,
    pub temperature: String,
    pub condition: String,
    pub humidity: String,
    pub error: String,
}

fn get_json(endpoint: &str, params: &[String]) -> Result<Value, String> {
    let mut command = Command::new("/usr/bin/curl");
    command.args(["-fsS", "--max-time", "8", "--get"]);
    for param in params { command.args(["--data-urlencode", param]); }
    let output = command.arg(endpoint).output().map_err(|err| err.to_string())?;
    if !output.status.success() { return Err("Weather service is unavailable".into()); }
    serde_json::from_slice(&output.stdout).map_err(|err| err.to_string())
}

fn condition(code: i64) -> &'static str {
    match code {
        0 => "Clear sky",
        1..=3 => "Partly cloudy",
        45 | 48 => "Fog",
        51..=57 => "Drizzle",
        61..=67 | 80..=82 => "Rain",
        71..=77 | 85 | 86 => "Snow",
        95..=99 => "Thunderstorm",
        _ => "Current conditions",
    }
}

pub fn fetch(city: &str) -> Weather {
    let city = city.trim();
    if city.chars().count() < 2 { return Weather { error: "Enter a city in Settings".into(), ..Default::default() }; }
    let result = (|| {
        let locations = get_json("https://geocoding-api.open-meteo.com/v1/search",
            &[format!("name={city}"), "count=1".into(), "format=json".into()])?;
        let place = locations["results"].as_array().and_then(|rows| rows.first())
            .ok_or("City not found")?;
        let lat = place["latitude"].as_f64().ok_or("Invalid location")?;
        let lon = place["longitude"].as_f64().ok_or("Invalid location")?;
        let forecast = get_json("https://api.open-meteo.com/v1/forecast",
            &[format!("latitude={lat}"), format!("longitude={lon}"),
              "current=temperature_2m,relative_humidity_2m,weather_code".into()])?;
        let current = &forecast["current"];
        let temp = current["temperature_2m"].as_f64().ok_or("No current weather")?;
        let humidity = current["relative_humidity_2m"].as_i64().unwrap_or(0);
        Ok::<_, String>(Weather {
            place: place["name"].as_str().unwrap_or(city).into(),
            temperature: format!("{temp:.0}°C"),
            condition: condition(current["weather_code"].as_i64().unwrap_or(-1)).into(),
            humidity: format!("{humidity}% humidity"),
            error: String::new(),
        })
    })();
    result.unwrap_or_else(|error| Weather { error, ..Default::default() })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn groups_weather_codes() {
        assert_eq!(condition(0), "Clear sky");
        assert_eq!(condition(63), "Rain");
        assert_eq!(condition(86), "Snow");
        assert_eq!(condition(95), "Thunderstorm");
    }
}
