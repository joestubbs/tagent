use std::path::PathBuf;

use config::Config;
use jwt_simple::algorithms::RS256PublicKey;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};

use crate::representations::TagentError;

// Tapis Tenants API response structs ---

#[derive(Debug, Serialize, Deserialize)]
struct TapisTenantsResult {
    public_key: String,
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TenantsAPIResponse {
    result: TapisTenantsResult,
}

// fetch the public key from a GET request to a uri.
// In pratcice, uri will be the Tapis tenants API endpoint; e.g.,
// uri = https://admin.tapis.io/v3/tenants/admin
async fn fetch_publickey(uri: &str) -> Result<String, String> {
    let res = reqwest::get(uri).await;
    let res = match res {
        Ok(response) => response,
        Err(error) => {
            let msg = format!(
                "Got error from GET request to Tenants API to retrieve public key. error: {}",
                error
            );
            error!("{}", msg);
            return Err(msg);
        }
    };
    match res.status() {
        reqwest::StatusCode::OK => {
            debug!("got 200 from request to fetch public key");
            match res.json::<TenantsAPIResponse>().await {
                Ok(response) => {
                    let public_key = response.result.public_key;
                    info!(
                        "Parsed JSON response from tenants API; public_key: {:?}",
                        public_key
                    );
                    Ok(public_key)
                }
                Err(error) => {
                    let msg = format!(
                        "Could not parse the JSON response from the tenants API; err: {}",
                        error
                    );
                    error!("{}", msg);
                    Err(msg)
                }
            }
        }
        _ => {
            let msg = format!(
                "did not get 200 from request to fetch public key; status: {}",
                res.status()
            );
            error!("{}", msg);
            Err(msg)
        }
    }
}

// Fetch the public key to use for signature verifaction from the Tenants API,
// by using the URL defined in the TAGENT_PUB_KEY_URL variable.
async fn fetch_pub_key_str_from_vars() -> std::io::Result<String> {
    let pub_key_url = std::env::var("TAGENT_PUB_KEY_URL");
    let pub_key_url = match pub_key_url {
        Ok(t) => {
            info!("TAGENT_PUB_KEY_URL was set to {}.", t);
            t
        }
        _ => {
            let msg = "Could not determine public key; must specify one of TAGENT_PUB_KEY or TAGENT_PUB_KEY_URL";
            error!("{}", msg);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
        }
    };
    let pub_key = fetch_publickey(&pub_key_url).await;
    let pub_key = match pub_key {
        Ok(p) => p,
        Err(e) => {
            let msg = format!(
                "Got error trying to fetch the public key from URL: {}; details: {}",
                pub_key_url, e
            );
            error!("{}", msg);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
        }
    };
    Ok(pub_key)
}

// Checks for the presence of envrionment variables to determine whether to retrieve the public key from the
// Tapis Tenants API or to get the public key from the environment.
async fn get_public_key_str() -> std::io::Result<String> {
    // if a public key is passed in directly as an environment variable, use that
    let pub_key_str = std::env::var("TAGENT_PUB_KEY");
    match pub_key_str {
        Ok(p) => {
            info!("TAGENT_PUB_KEY was set.");
            Ok(p)
        }
        _ => {
            // otherwise, get the public key from the Tenants API
            return fetch_pub_key_str_from_vars().await;
        }
    }
}

// RSA256 PEM PKS#8 format requires line breaks at the 64 character mark;
// cf., https://www.rfc-editor.org/rfc/rfc1421, section 4.3.2.4  Step 4: Printable Encoding
//     "...with each line except the last containing exactly 64 printable characters and the final line containing
//      64 or fewer printable characters."
// This function adds the necessary line breaks
fn insert_line_breaks_pub_key(pub_key: String) -> std::io::Result<String> {
    let mut result = pub_key;
    // first location of a required newline
    let mut idx = 26;
    while idx < result.len() {
        // get the character at the next endline position, bubble up None
        let t = result.get(idx..idx + 1);
        let t = match t {
            Some(t) => t,
            _ => {
                let msg = "Unexpected error inserting newlines to public key";
                error!("{}", msg);
                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
            }
        };
        if t != "\n" {
            result.insert(idx, '\n');
        };
        idx += 65;
    }
    Ok(result)
}

// Public function for calculating the public key to use for signature verification.
pub async fn get_pub_key() -> std::io::Result<RS256PublicKey> {
    let pk_str = get_public_key_str().await?;
    let pk_str = insert_line_breaks_pub_key(pk_str)?;
    let rsa_pub_key = RS256PublicKey::from_pem(&pk_str);
    let rsa_pub_key = match rsa_pub_key {
        Ok(key) => key,
        Err(error) => {
            let msg = format!(
                "Error generating RSAPublicKey from pub_key string; err: {}",
                error
            );
            error!("{}", msg);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
        }
    };
    Ok(rsa_pub_key)
}

// Configuration management
// ========================

const CONFIG_FILE: &str = "tagent/settings.yaml";
const VAR_PREFIX: &str = "TAGENT";

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TagentConfig {
    pub root_directory: PathBuf,
    pub public_key: Option<String>,
    pub address: String,
    pub port: i16,
}

impl TagentConfig {
    pub fn new() -> Result<Self, TagentError> {
        Ok(TagentConfig {
            root_directory: dirs::home_dir().ok_or("couldn't get user's home directory")?,
            public_key: None,
            address: String::from("127.0.0.1"),
            port: 8080,
        })
    }
}

impl From<config::ConfigError> for TagentError {
    fn from(config_error: config::ConfigError) -> Self {
        TagentError::new_with_version(format!("Configuration error: {}", config_error))
    }
}

impl TagentConfig {
    pub fn from_sources() -> Result<Self, TagentError> {
        let mut config = dirs::config_dir().ok_or("couldn't get config directory")?;
        config.push(CONFIG_FILE);
        let config_path = config
            .to_str()
            .ok_or("path to config file cannot be converted to string")?;
        Self::from_sources_with_names(config_path, VAR_PREFIX)
    }

    fn from_sources_with_names(file: &str, var_prefix: &str) -> Result<Self, TagentError> {
        let settings = Config::builder()
            .add_source(config::Config::try_from::<TagentConfig>(
                &TagentConfig::new()?,
            )?)
            .add_source(config::File::with_name(file).required(false))
            .add_source(config::Environment::with_prefix(var_prefix))
            .build()?
            .try_deserialize::<TagentConfig>()?;
        Ok(settings)
    }
}

#[cfg(test)]
mod test {
    use std::io::Write;

    use super::*;

    #[actix_rt::test]
    async fn get_pub_key_with_key_var() -> std::io::Result<()> {
        std::env::set_var("TAGENT_PUB_KEY", "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAneCSAKpfuRxX7DpuBBoYEhIayF6yGppgR3I6jO1cvN0+6gc36wHo3O93bnfNl2cYSmbpp9dtd1T2Uv1t5DCGe+s2bd/VwfO6IgMu2GuHHkQcTqTJb0axIJftqo5lfopSOvyeN5oEo/ti7fw0hCdzArQhcTtkaU4m5spL7+5XUOnFiwPZB+unxGVVQ5rmI9TVW74iNZ4ESlzRTp2VT0sZ6QIIOBZA2kLx+fgg3YQuZpZ4rz6oJ8zyWEik+v14Rm6AUBI1XTyVXDr2KJZpXJ5cVCW/xIua4Z97woKZJ1qk7rL/PrN2iT7/6bM35rVFU3kTvZKfXRPTE8ZWTiWGWAFu+QIDAQAB\n-----END PUBLIC KEY-----");
        let _a = get_pub_key().await.unwrap();

        Ok(())
    }

    #[actix_rt::test]
    async fn get_pub_key_with_url_var() -> std::io::Result<()> {
        std::env::remove_var("TAGENT_PUB_KEY");
        std::env::set_var(
            "TAGENT_PUB_KEY_URL",
            "https://admin.tapis.io/v3/tenants/admin",
        );
        // note: this test can fail if Tapis API is not available..
        let _a = get_pub_key().await.unwrap();

        Ok(())
    }

    #[actix_rt::test]
    async fn get_pub_key_should_fail_if_no_vars() -> std::io::Result<()> {
        std::env::remove_var("TAGENT_PUB_KEY");
        std::env::remove_var("TAGENT_PUB_KEY_URL");
        let a = get_pub_key().await;
        assert!(a.is_err());
        Ok(())
    }

    #[test]
    fn config_defaults_should_load_first() -> std::io::Result<()> {
        let temp = tempfile::TempDir::new()?;
        // Ensure no file or variables exist, for obtaining the default values
        let file = temp.path().join("foo.yaml");
        let filename = file.to_str().unwrap();
        let prefix = uuid::Uuid::new_v4().to_string();
        let config = TagentConfig::from_sources_with_names(filename, &prefix)?;
        assert_eq!(config, TagentConfig::new()?);
        Ok(())
    }

    #[test]
    fn config_should_read_file_if_it_exists() -> std::io::Result<()> {
        let temp = tempfile::TempDir::new()?;
        let filename = temp.path().join("foo.yaml");
        let mut file = std::fs::File::create(&filename)?;
        let contents = "root_directory: foo\nport: 12";
        file.write_all(contents.as_bytes())?;
        let prefix = uuid::Uuid::new_v4().to_string();
        let config = TagentConfig::from_sources_with_names(filename.to_str().unwrap(), &prefix)?;
        assert_eq!(config.root_directory.to_str().unwrap(), "foo");
        assert_eq!(config.port, 12);
        Ok(())
    }

    #[test]
    fn config_should_read_environment_variables() -> std::io::Result<()> {
        let temp = tempfile::TempDir::new()?;
        // Ensure no file or variables exist, for obtaining the default values
        let file = temp.path().join("foo.yaml");
        let filename = file.to_str().unwrap();
        let prefix = uuid::Uuid::new_v4().to_string();
        std::env::set_var(format!("{}_ROOT_DIRECTORY", &prefix), "bar");
        std::env::set_var(format!("{}_PORT", &prefix), "15");
        let config = TagentConfig::from_sources_with_names(filename, &prefix)?;
        assert_eq!(config.root_directory.to_str().unwrap(), "bar");
        assert_eq!(config.port, 15);
        Ok(())
    }
}
