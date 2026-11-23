use serde::Deserialize;
use tether_vcs::HttpResponse;

use super::{CiError, JobRef, JobStep, RunRef, RunSummary, WorkflowMissing};

#[derive(Debug, Deserialize)]
pub(super) struct RunPage {
    pub(super) workflow_runs: Vec<ApiRun>,
}

#[derive(Debug, Deserialize)]
pub(super) struct JobPage {
    pub(super) jobs: Vec<ApiJob>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ApiRun {
    pub(super) id: u64,
    pub(super) run_number: Option<u64>,
    pub(super) display_title: String,
    pub(super) head_branch: Option<String>,
    pub(super) status: String,
    pub(super) conclusion: Option<String>,
    pub(super) created_at: String,
    pub(super) run_started_at: Option<String>,
    pub(super) updated_at: Option<String>,
    pub(super) html_url: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ApiJob {
    id: u64,
    name: String,
    status: String,
    conclusion: Option<String>,
    steps: Option<Vec<ApiStep>>,
}

#[derive(Debug, Deserialize)]
struct ApiStep {
    name: String,
    status: String,
    conclusion: Option<String>,
}

impl From<ApiRun> for RunRef {
    fn from(run: ApiRun) -> Self {
        Self {
            id: run.id,
            display_title: run.display_title,
            branch: run.head_branch.unwrap_or_default(),
            status: run.status,
            conclusion: run.conclusion,
            created_at: run.created_at,
            html_url: run.html_url,
        }
    }
}

impl From<ApiJob> for JobRef {
    fn from(job: ApiJob) -> Self {
        Self {
            id: job.id,
            name: job.name,
            status: job.status,
            conclusion: job.conclusion,
            steps: job
                .steps
                .unwrap_or_default()
                .into_iter()
                .map(|step| JobStep {
                    name: step.name,
                    status: step.status,
                    conclusion: step.conclusion,
                })
                .collect(),
        }
    }
}

impl From<ApiRun> for RunSummary {
    fn from(run: ApiRun) -> Self {
        let duration_seconds =
            duration_seconds(run.run_started_at.as_deref(), run.updated_at.as_deref());
        let started_at = run.run_started_at.clone().or(Some(run.created_at.clone()));
        Self {
            id: run.id,
            number: run.run_number.unwrap_or_default(),
            title: run
                .display_title
                .strip_prefix("tether ")
                .unwrap_or(&run.display_title)
                .to_owned(),
            branch: run.head_branch.unwrap_or_default(),
            status: run.status,
            conclusion: run.conclusion,
            started_at,
            duration_seconds,
            html_url: run.html_url,
        }
    }
}

fn duration_seconds(start: Option<&str>, end: Option<&str>) -> Option<u64> {
    Some(parse_rfc3339(end?)?.saturating_sub(parse_rfc3339(start?)?))
}

pub(super) fn parse_rfc3339(value: &str) -> Option<u64> {
    let bytes = value.as_bytes();
    let year = number_at(bytes, 0)?
        .saturating_mul(100)
        .saturating_add(number_at(bytes, 2)?);
    let month = number_at(bytes, 5)?;
    let day = number_at(bytes, 8)?;
    let hour = number_at(bytes, 11)?;
    let minute = number_at(bytes, 14)?;
    let second = number_at(bytes, 17)?;
    let days = days_before_year(year) + days_before_month(year, month) + day.saturating_sub(1);
    Some(
        days.saturating_sub(days_before_year(1970))
            .saturating_mul(86_400)
            .saturating_add(hour.saturating_mul(3_600))
            .saturating_add(minute.saturating_mul(60))
            .saturating_add(second),
    )
}

fn number_at(bytes: &[u8], index: usize) -> Option<u64> {
    let first = *bytes.get(index)?;
    let second = *bytes.get(index + 1)?;
    first
        .is_ascii_digit()
        .then_some(u64::from(first - b'0'))
        .zip(second.is_ascii_digit().then_some(u64::from(second - b'0')))
        .map(|(first, second)| first * 10 + second)
}

fn days_before_year(year: u64) -> u64 {
    let years = year.saturating_sub(1);
    years * 365 + years / 4 - years / 100 + years / 400
}

fn days_before_month(year: u64, month: u64) -> u64 {
    const DAYS: [u64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let base = DAYS
        .get(
            usize::try_from(month.saturating_sub(1))
                .ok()
                .unwrap_or_default(),
        )
        .copied()
        .unwrap_or_default();
    base + u64::from(month > 2 && is_leap_year(year))
}

fn is_leap_year(year: u64) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

pub(super) fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

pub(super) fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = chunk.get(1).copied().unwrap_or_default();
        let c = chunk.get(2).copied().unwrap_or_default();
        output.push(TABLE[(a >> 2) as usize] as char);
        output.push(TABLE[((a & 3) << 4 | b >> 4) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[((b & 15) << 2 | c >> 6) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

pub(super) fn map_response(response: &HttpResponse, token: &str) -> Result<(), CiError> {
    match response.status {
        200..=299 | 302 => Ok(()),
        401 => Err(CiError::BadCredentials),
        403 if response
            .headers
            .get("x-ratelimit-remaining")
            .map(String::as_str)
            == Some("0") =>
        {
            Err(CiError::RateLimited {
                reset_at: response
                    .headers
                    .get("x-ratelimit-reset")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or_default(),
            })
        }
        403 => Err(CiError::Forbidden {
            message: redact(github_message(response), token),
        }),
        404 if response
            .body
            .windows(8)
            .any(|window| window.eq_ignore_ascii_case(b"workflow")) =>
        {
            Err(CiError::WorkflowMissing(WorkflowMissing))
        }
        404 => Err(CiError::NotFound),
        422 => Err(CiError::Validation {
            messages: validation_messages(response)
                .into_iter()
                .map(|message| redact(message, token))
                .collect(),
        }),
        500..=599 => Err(CiError::Retryable),
        _ => Err(CiError::InvalidResponse),
    }
}

fn github_message(response: &HttpResponse) -> String {
    serde_json::from_slice::<serde_json::Value>(&response.body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(|message| message.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "request denied".into())
}

fn validation_messages(response: &HttpResponse) -> Vec<String> {
    serde_json::from_slice::<serde_json::Value>(&response.body)
        .ok()
        .and_then(|value| {
            value
                .get("errors")
                .and_then(|errors| errors.as_array())
                .map(|items| items.iter().filter_map(item_message).collect())
        })
        .filter(|messages: &Vec<String>| !messages.is_empty())
        .unwrap_or_else(|| vec![github_message(response)])
}

fn item_message(item: &serde_json::Value) -> Option<String> {
    item.get("message")
        .and_then(|value| value.as_str())
        .map(str::to_owned)
}

fn redact(message: String, token: &str) -> String {
    if token.is_empty() {
        return message;
    }
    message.replace(token, "[redacted]")
}
