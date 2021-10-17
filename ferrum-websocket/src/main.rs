use ferrum_shared::settings::get_settings;
use ferrum_websocket::application::Application;

#[cfg(not(tarpaulin))]
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let settings = get_settings().expect("Failed to read settings");
    let application = Application::build(settings).await?;

    application.run_until_stopped().await;

    Ok(())
}

#[cfg(tarpaulin)]
fn main() {}
