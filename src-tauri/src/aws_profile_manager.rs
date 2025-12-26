use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use dirs::home_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsProfile {
    pub name: String,
    pub region: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub profile_source: ProfileSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfileSource {
    CredentialsFile,
    ConfigFile,
    Environment,
    IamRole,
}

pub struct AwsProfileManager;

impl AwsProfileManager {
    pub fn new() -> Self {
        Self
    }

    /// Get the default AWS credentials file path
    pub fn get_credentials_file_path() -> Option<PathBuf> {
        home_dir().map(|home| home.join(".aws").join("credentials"))
    }

    /// Get the AWS config file path
    pub fn get_config_file_path() -> Option<PathBuf> {
        home_dir().map(|home| home.join(".aws").join("config"))
    }

    /// Load all available AWS profiles from system
    pub fn load_available_profiles() -> Result<Vec<AwsProfile>, String> {
        let mut profiles = Vec::new();

        // Load from credentials file
        if let Some(creds_path) = Self::get_credentials_file_path() {
            if creds_path.exists() {
                if let Ok(creds_profiles) = Self::parse_credentials_file(&creds_path) {
                    profiles.extend(creds_profiles);
                }
            }
        }

        // Load from config file
        if let Some(config_path) = Self::get_config_file_path() {
            if config_path.exists() {
                if let Ok(config_profiles) = Self::parse_config_file(&config_path) {
                    profiles.extend(config_profiles);
                }
            }
        }

        // Check for environment variables (default profile)
        if let Ok(env_profile) = Self::load_environment_profile() {
            profiles.push(env_profile);
        }

        Ok(profiles)
    }

    /// Parse AWS credentials file
    fn parse_credentials_file(path: &PathBuf) -> Result<Vec<AwsProfile>, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read credentials file: {}", e))?;

        let mut profiles = Vec::new();
        let mut current_profile: Option<AwsProfile> = None;

        for line in content.lines() {
            let line = line.trim();
            
            if line.starts_with('[') && line.ends_with(']') {
                // Save previous profile if exists
                if let Some(profile) = current_profile.take() {
                    profiles.push(profile);
                }
                
                // Start new profile
                let profile_name = line[1..line.len()-1].to_string();
                if profile_name.starts_with("profile ") {
                    let name = profile_name[8..].to_string();
                    current_profile = Some(AwsProfile {
                        name,
                        region: None,
                        access_key_id: None,
                        secret_access_key: None,
                        session_token: None,
                        profile_source: ProfileSource::CredentialsFile,
                    });
                } else if profile_name == "default" {
                    current_profile = Some(AwsProfile {
                        name: "default".to_string(),
                        region: None,
                        access_key_id: None,
                        secret_access_key: None,
                        session_token: None,
                        profile_source: ProfileSource::CredentialsFile,
                    });
                }
            } else if let Some(ref mut profile) = current_profile {
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    
                    match key {
                        "aws_access_key_id" => profile.access_key_id = Some(value.to_string()),
                        "aws_secret_access_key" => profile.secret_access_key = Some(value.to_string()),
                        "aws_session_token" => profile.session_token = Some(value.to_string()),
                        _ => {}
                    }
                }
            }
        }

        // Save last profile
        if let Some(profile) = current_profile {
            profiles.push(profile);
        }

        Ok(profiles)
    }

    /// Parse AWS config file
    fn parse_config_file(path: &PathBuf) -> Result<Vec<AwsProfile>, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        let mut profiles = Vec::new();
        let mut current_profile: Option<AwsProfile> = None;

        for line in content.lines() {
            let line = line.trim();
            
            if line.starts_with('[') && line.ends_with(']') {
                // Save previous profile if exists
                if let Some(profile) = current_profile.take() {
                    profiles.push(profile);
                }
                
                // Start new profile
                let profile_name = line[1..line.len()-1].to_string();
                if profile_name.starts_with("profile ") {
                    let name = profile_name[8..].to_string();
                    current_profile = Some(AwsProfile {
                        name,
                        region: None,
                        access_key_id: None,
                        secret_access_key: None,
                        session_token: None,
                        profile_source: ProfileSource::ConfigFile,
                    });
                }
            } else if let Some(ref mut profile) = current_profile {
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    
                    match key {
                        "region" => profile.region = Some(value.to_string()),
                        "credential_process" => {
                            // For credential_process, we don't execute it here
                            // Just mark that this profile uses a credential process
                        },
                        "source_profile" => {
                            // Handle source_profile for role assumption
                        },
                        "role_arn" => {
                            // Mark as IAM role profile
                            profile.profile_source = ProfileSource::IamRole;
                        },
                        _ => {}
                    }
                }
            }
        }

        // Save last profile
        if let Some(profile) = current_profile {
            profiles.push(profile);
        }

        Ok(profiles)
    }

    /// Load profile from environment variables
    fn load_environment_profile() -> Result<AwsProfile, String> {
        let access_key_id = std::env::var("AWS_ACCESS_KEY_ID").ok();
        let secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
        let session_token = std::env::var("AWS_SESSION_TOKEN").ok();
        let region = std::env::var("AWS_DEFAULT_REGION").ok()
            .or_else(|| std::env::var("AWS_REGION").ok());

        if access_key_id.is_some() || secret_access_key.is_some() {
            Ok(AwsProfile {
                name: "environment".to_string(),
                region,
                access_key_id,
                secret_access_key,
                session_token,
                profile_source: ProfileSource::Environment,
            })
        } else {
            Err("No AWS credentials found in environment".to_string())
        }
    }

    /// Test if a profile is valid by attempting to use it
    pub fn test_profile(&self, profile: &AwsProfile) -> Result<bool, String> {
        // For now, just check if we have the required credentials
        // In a real implementation, you would use AWS SDK to validate
        
        match profile.profile_source {
            ProfileSource::Environment => {
                if profile.access_key_id.is_some() && profile.secret_access_key.is_some() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            ProfileSource::CredentialsFile => {
                if profile.access_key_id.is_some() && profile.secret_access_key.is_some() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            ProfileSource::ConfigFile => {
                // Config file profiles might reference credentials file
                // or use IAM roles, so we can't validate without AWS SDK
                Ok(true) // Assume valid for now
            },
            ProfileSource::IamRole => {
                // IAM role profiles are valid if they have role_arn
                Ok(true) // Assume valid for now
            },
        }
    }

    /// Get profile by name
    pub fn get_profile_by_name(profile_name: &str) -> Result<Option<AwsProfile>, String> {
        let profiles = Self::load_available_profiles()?;
        Ok(profiles.into_iter().find(|p| p.name == profile_name))
    }
}

impl Default for AwsProfileManager {
    fn default() -> Self {
        Self::new()
    }
}
