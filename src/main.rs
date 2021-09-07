use ferrum::{application::Application, settings::get_settings};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let settings = get_settings().expect("Failed to read settings");
    let application = Application::build(settings).await?;

    application.run_until_stopped().await?;

    Ok(())
}
