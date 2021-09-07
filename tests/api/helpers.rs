use ferrum::{
    application::Application,
    settings::get_settings,
    telemetry::{get_subscriber, init_subscriber},
};
use once_cell::sync::Lazy;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber)
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber)
    };
});

pub struct TestApplication {
    pub address: String,
    pub port: u16,
}

pub async fn spawn_app() -> TestApplication {
    Lazy::force(&TRACING);

    let settings = {
        let mut settings = get_settings().expect("Failed to read settings");

        settings.application.port = 0;

        settings
    };

    let application = Application::build(settings.clone())
        .await
        .expect("Failed to build application");

    let application_port = application.port();

    let _ = tokio::spawn(application.run_until_stopped());

    TestApplication {
        address: format!("http://localhost:{}", application_port),
        port: application_port,
    }
}
