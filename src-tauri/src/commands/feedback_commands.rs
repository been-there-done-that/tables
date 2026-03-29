use serde::{Deserialize, Serialize};
use sysinfo::System;
use tauri::AppHandle;

/// Returned by get_system_info and embedded in FeedbackPayload for bug reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub memory_gb: u64,
}

/// Payload sent from the frontend to submit_feedback.
#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackPayload {
    pub feedback_type: String,
    pub title: Option<String>,
    pub body: String,
    pub steps: Option<String>,
    pub system_info: Option<SystemInfo>,
}

/// URL of the Cloudflare Worker that proxies submissions to GitHub Issues.
/// Replace with the real deployed Worker URL before shipping.
const FEEDBACK_WORKER_URL: &str = "https://tables-feedback.OWNER.workers.dev";

#[tauri::command]
pub async fn get_system_info(app: AppHandle) -> Result<SystemInfo, String> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let version = app.package_info().version.to_string();
    let os = format!(
        "{} {}",
        System::name().unwrap_or_else(|| "Unknown OS".to_string()),
        System::os_version().unwrap_or_else(|| String::new())
    )
    .trim()
    .to_string();
    let arch = std::env::consts::ARCH.to_string();
    let memory_gb = sys.total_memory() / (1024 * 1024 * 1024);

    Ok(SystemInfo {
        version,
        os,
        arch,
        memory_gb,
    })
}

#[tauri::command]
pub async fn submit_feedback(payload: FeedbackPayload) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .post(FEEDBACK_WORKER_URL)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() || e.is_timeout() {
                "Could not connect. Check your internet connection.".to_string()
            } else {
                "Submission failed. Please try again later.".to_string()
            }
        })?;

    if response.status().is_success() {
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Invalid response: {}", e))?;
        body["issue_url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Submission succeeded but no issue URL was returned.".to_string())
    } else {
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|_| "Submission failed. Try again later.".to_string())?;
        Err(body["error"]
            .as_str()
            .unwrap_or("Submission failed. Try again later.")
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feedback_payload_serializes_correctly() {
        let payload = FeedbackPayload {
            feedback_type: "bug".to_string(),
            title: Some("Test bug".to_string()),
            body: "Something went wrong".to_string(),
            steps: None,
            system_info: Some(SystemInfo {
                version: "0.1.0".to_string(),
                os: "macOS 15.4".to_string(),
                arch: "aarch64".to_string(),
                memory_gb: 16,
            }),
        };

        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["feedback_type"], "bug");
        assert_eq!(json["title"], "Test bug");
        assert_eq!(json["body"], "Something went wrong");
        assert!(json["steps"].is_null());
        assert_eq!(json["system_info"]["memory_gb"], 16);
    }

    #[test]
    fn feedback_payload_optional_fields_nullable() {
        let payload = FeedbackPayload {
            feedback_type: "feedback".to_string(),
            title: None,
            body: "Great app".to_string(),
            steps: None,
            system_info: None,
        };

        let json = serde_json::to_value(&payload).unwrap();
        assert!(json["title"].is_null());
        assert!(json["system_info"].is_null());
    }
}
