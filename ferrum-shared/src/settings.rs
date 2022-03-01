// © https://github.com/LukeMathWalker/zero-to-production

use std::{
    convert::{TryFrom, TryInto},
    time::Duration,
};

use config::{Config, ConfigError};
use serde::Deserialize;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    PgPool,
};

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub broker: BrokerSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApplicationSettings {
    pub host: String,
    pub port: u16,
    pub base_url: String,
    pub jwt_secret: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub database_name: String,
    pub require_ssl: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BrokerSettings {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub queue: String,
}

impl DatabaseSettings {
    ///
    /// Get [PgConnectOptions](sqlx::postgres::PgConnectOptions) without a specific database.
    ///
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        PgConnectOptions::new()
            .username(&self.username)
            .password(&self.password)
            .host(&self.host)
            .port(self.port)
            .ssl_mode(ssl_mode)
    }

    ///
    /// Get [PgConnectOptions](sqlx::postgres::PgConnectOptions) with the database specified in the settings.
    ///
    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db().database(&self.database_name)
    }
}

impl BrokerSettings {
    pub fn get_connection_string(&self) -> String {
        format!(
            "amqp://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}

///
/// Available settings environments
///
#[derive(Debug)]
pub enum Environment {
    Local,
    Production,
    Testing,
}

impl Environment {
    ///
    /// Get the string representation for an enum.
    /// This can be used to load the settings files.
    ///
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
            Environment::Testing => "testing",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            "testing" => Ok(Self::Testing),
            other => Err(format!("Unknown environment {:?}!", other)),
        }
    }
}

///
/// Get an instance of the settings.
///
/// This uses the current `APP_ENV` to dertermine the settings file to load.
///
pub fn get_settings() -> Result<Settings, ConfigError> {
    let base_path = std::env::current_dir().expect("Error while getting current directory");
    let settings_directory = base_path.join("./settings");

    let environment: Environment = std::env::var("APP_ENV")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENV");

    let builder = Config::builder()
        .add_source(config::File::from(settings_directory.join("base")).required(true))
        .add_source(
            config::File::from(settings_directory.join(environment.as_str())).required(true),
        )
        .add_source(config::Environment::with_prefix("app").separator("__"));

    match builder.build() {
        Ok(config) => config.try_deserialize(),
        Err(error) => panic!("Error building config: {error:?}"),
    }
}

///
/// Get a [`PgPool`] from the supplied [`DatabaseSettings`].
///
/// Also sets a default timeout of 5 seconds.
///
pub async fn get_db_pool(settings: &DatabaseSettings) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .connect_timeout(Duration::from_secs(5))
        .connect_with(settings.with_db())
        .await
}

#[cfg(test)]
mod tests {
    use claim::{assert_err, assert_ok};

    use super::*;

    #[test]
    fn valid_environment_variable_are_accepted() {
        for env in ["local", "production", "testing"] {
            let env: Result<Environment, String> = env.to_string().try_into();

            assert_ok!(env);
        }
    }

    #[test]
    fn invalid_environment_variable_is_rejected() {
        let env: Result<Environment, String> = "foobar".to_string().try_into();

        assert_err!(env);
    }

    #[test]
    #[should_panic]
    fn get_settings_fails_for_invalid_app_env_format() {
        std::env::set_var("APP_ENV", "foobar");

        get_settings().unwrap();
    }
}
