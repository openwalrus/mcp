//! MCP server providing timezone-aware time tools.
//!
//! Implements two tools following the MCP time server reference:
//! - `get_current_time`: Get current time in any IANA timezone
//! - `convert_time`: Convert time between timezones

use chrono::{Datelike, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo},
    schemars::{self, JsonSchema},
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

/// Parameters for the `get_current_time` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetCurrentTimeParams {
    /// IANA timezone name (e.g. "America/New_York", "Europe/London", "UTC").
    pub timezone: String,
}

/// Parameters for the `convert_time` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConvertTimeParams {
    /// Source IANA timezone name.
    pub source_timezone: String,
    /// Time in 24-hour format (HH:MM).
    pub time: String,
    /// Target IANA timezone name.
    pub target_timezone: String,
}

/// Result for a single timezone time query.
#[derive(Debug, Serialize)]
pub struct TimeResult {
    /// The IANA timezone name.
    pub timezone: String,
    /// ISO 8601 datetime string with offset.
    pub datetime: String,
    /// Day of the week (e.g. "Monday").
    pub day_of_week: String,
    /// Whether daylight saving time is in effect.
    pub is_dst: bool,
}

/// Result for a timezone conversion.
#[derive(Debug, Serialize)]
pub struct ConvertTimeResult {
    /// Time in the source timezone.
    pub source: TimeResult,
    /// Time in the target timezone.
    pub target: TimeResult,
    /// Human-readable time difference (e.g. "+5.0h", "-3.5h").
    pub time_difference: String,
}

fn parse_tz(name: &str) -> Result<Tz, String> {
    name.parse::<Tz>()
        .map_err(|_| format!("Invalid timezone: {name}"))
}

fn time_result(tz: Tz, dt: chrono::DateTime<Tz>) -> TimeResult {
    let offset_secs = dt.offset().fix().local_minus_utc();

    // Determine DST by comparing current offset to the offset on Jan 1 (winter).
    let jan1 = tz
        .with_ymd_and_hms(dt.year(), 1, 1, 0, 0, 0)
        .single()
        .map(|d| d.offset().fix().local_minus_utc());
    let is_dst = jan1.is_some_and(|std_offset| offset_secs != std_offset);

    TimeResult {
        timezone: tz.to_string(),
        datetime: dt.to_rfc3339(),
        day_of_week: dt.format("%A").to_string(),
        is_dst,
    }
}

/// MCP time server.
#[derive(Debug, Clone)]
pub struct TimeServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl TimeServer {
    /// Create a new time server.
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Get the current time in a specific IANA timezone.
    #[tool(description = "Get the current time in a specific timezone")]
    async fn get_current_time(
        &self,
        Parameters(params): Parameters<GetCurrentTimeParams>,
    ) -> Result<String, String> {
        let tz = parse_tz(&params.timezone)?;
        let now = Utc::now().with_timezone(&tz);
        let result = time_result(tz, now);
        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
    }

    /// Convert a time from one timezone to another.
    #[tool(description = "Convert time between timezones")]
    async fn convert_time(
        &self,
        Parameters(params): Parameters<ConvertTimeParams>,
    ) -> Result<String, String> {
        let source_tz = parse_tz(&params.source_timezone)?;
        let target_tz = parse_tz(&params.target_timezone)?;

        // Parse HH:MM
        let parts: Vec<&str> = params.time.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid time format. Expected HH:MM (24-hour format)".into());
        }
        let hour: u32 = parts[0]
            .parse()
            .map_err(|_| "Invalid hour in time".to_string())?;
        let minute: u32 = parts[1]
            .parse()
            .map_err(|_| "Invalid minute in time".to_string())?;
        if hour >= 24 || minute >= 60 {
            return Err("Invalid time. Hour must be 0-23, minute must be 0-59".into());
        }

        // Build datetime in source timezone using today's date
        let now_utc = Utc::now();
        let now_source = now_utc.with_timezone(&source_tz);
        let source_dt = source_tz
            .with_ymd_and_hms(
                now_source.year(),
                now_source.month(),
                now_source.day(),
                hour,
                minute,
                0,
            )
            .single()
            .ok_or("Ambiguous or invalid time in source timezone")?;

        let target_dt = source_dt.with_timezone(&target_tz);

        // Calculate offset difference in hours
        let source_offset = source_dt.offset().fix().local_minus_utc() as f64 / 3600.0;
        let target_offset = target_dt.offset().fix().local_minus_utc() as f64 / 3600.0;
        let diff = target_offset - source_offset;
        let sign = if diff >= 0.0 { "+" } else { "" };
        let time_difference = if diff.fract() == 0.0 {
            format!("{sign}{diff:.1}h")
        } else {
            format!("{sign}{}h", diff)
        };

        let result = ConvertTimeResult {
            source: time_result(source_tz, source_dt),
            target: time_result(target_tz, target_dt),
            time_difference,
        };
        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
    }
}

impl Default for TimeServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler]
impl ServerHandler for TimeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "wmcp-time".into(),
                title: Some("Walrus MCP Time Server".into()),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            instructions: Some(
                "Time server providing timezone-aware current time and conversion tools.".into(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ConvertTimeParams, GetCurrentTimeParams, TimeServer};
    use rmcp::handler::server::wrapper::Parameters;

    #[tokio::test]
    async fn get_current_time_utc() {
        let server = TimeServer::new();
        let result = server
            .get_current_time(Parameters(GetCurrentTimeParams {
                timezone: "UTC".into(),
            }))
            .await;
        let text = result.expect("should succeed");
        assert!(text.contains("\"timezone\": \"UTC\""));
        assert!(text.contains("\"is_dst\": false"));
    }

    #[tokio::test]
    async fn get_current_time_invalid_tz() {
        let server = TimeServer::new();
        let result = server
            .get_current_time(Parameters(GetCurrentTimeParams {
                timezone: "Invalid/Zone".into(),
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid timezone"));
    }

    #[tokio::test]
    async fn convert_time_utc_to_tokyo() {
        let server = TimeServer::new();
        let result = server
            .convert_time(Parameters(ConvertTimeParams {
                source_timezone: "UTC".into(),
                time: "12:00".into(),
                target_timezone: "Asia/Tokyo".into(),
            }))
            .await;
        let text = result.expect("should succeed");
        assert!(text.contains("\"timezone\": \"UTC\""));
        assert!(text.contains("\"timezone\": \"Asia/Tokyo\""));
        assert!(text.contains("+9.0h"));
    }

    #[tokio::test]
    async fn convert_time_invalid_format() {
        let server = TimeServer::new();
        let result = server
            .convert_time(Parameters(ConvertTimeParams {
                source_timezone: "UTC".into(),
                time: "25:00".into(),
                target_timezone: "Asia/Tokyo".into(),
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn convert_time_bad_format() {
        let server = TimeServer::new();
        let result = server
            .convert_time(Parameters(ConvertTimeParams {
                source_timezone: "UTC".into(),
                time: "noon".into(),
                target_timezone: "Asia/Tokyo".into(),
            }))
            .await;
        assert!(result.is_err());
    }
}
