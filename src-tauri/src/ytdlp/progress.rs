use serde::Serialize;

use super::args::PROGRESS_PREFIX;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressUpdate {
    pub id: String,
    pub status: String,       // downloading | finished | error
    pub downloaded: u64,
    pub total: Option<u64>,   // None when unknown
    pub percent: Option<f32>, // 0..100
    pub speed_bps: Option<f64>,
    pub eta_secs: Option<u64>,
    pub title: Option<String>,
}

/// Parse a single yt-dlp stdout line into a ProgressUpdate.
/// Returns None if the line is not a progress line.
pub fn parse_line(id: &str, line: &str) -> Option<ProgressUpdate> {
    let line = line.trim_end_matches(['\r', '\n']);
    let rest = line.strip_prefix(PROGRESS_PREFIX)?;
    let rest = rest.strip_prefix('\t').unwrap_or(rest);
    let parts: Vec<&str> = rest.split('\t').collect();
    // status, downloaded, total, total_estimate, speed, eta, title
    if parts.len() < 7 {
        return None;
    }
    let status = parts[0].to_string();
    let downloaded = parse_u64(parts[1]).unwrap_or(0);
    let total = parse_u64(parts[2]).or_else(|| parse_u64(parts[3]));
    let speed_bps = parse_f64(parts[4]);
    let eta_secs = parse_u64(parts[5]);
    let title = if parts[6].is_empty() || parts[6] == "NA" {
        None
    } else {
        Some(parts[6].to_string())
    };
    let percent = match total {
        Some(t) if t > 0 => Some((downloaded as f32 / t as f32) * 100.0),
        _ => None,
    };

    Some(ProgressUpdate {
        id: id.to_string(),
        status,
        downloaded,
        total,
        percent,
        speed_bps,
        eta_secs,
        title,
    })
}

fn parse_u64(s: &str) -> Option<u64> {
    if s.is_empty() || s == "NA" {
        return None;
    }
    s.parse::<u64>().ok().or_else(|| s.parse::<f64>().ok().map(|v| v as u64))
}

fn parse_f64(s: &str) -> Option<f64> {
    if s.is_empty() || s == "NA" {
        return None;
    }
    s.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_progress_line() {
        let line = format!(
            "{}\tdownloading\t1048576\t10485760\tNA\t524288.0\t18\tSome Title",
            PROGRESS_PREFIX
        );
        let p = parse_line("abc", &line).unwrap();
        assert_eq!(p.id, "abc");
        assert_eq!(p.status, "downloading");
        assert_eq!(p.downloaded, 1_048_576);
        assert_eq!(p.total, Some(10_485_760));
        assert!((p.percent.unwrap() - 10.0).abs() < 0.01);
        assert_eq!(p.speed_bps, Some(524_288.0));
        assert_eq!(p.eta_secs, Some(18));
        assert_eq!(p.title.as_deref(), Some("Some Title"));
    }

    #[test]
    fn ignores_non_progress() {
        assert!(parse_line("abc", "[download] Destination: foo.mp4").is_none());
    }
}
