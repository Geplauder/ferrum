use ferrum::application::Application;

use ferrum_shared::{
    settings::{get_db_pool, get_settings},
    telemetry::{get_subscriber, init_subscriber},
};

#[cfg(not(tarpaulin))]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("ferrum".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let settings = get_settings().expect("Failed to read settings");

    {
        let db_pool = get_db_pool(&settings.database).await.unwrap();
        sqlx::migrate!("./ferrum-db/migrations")
            .run(&db_pool)
            .await
            .unwrap();
    }

    let application = Application::build(settings).await?;

    application.run_until_stopped().await?;

    Ok(())
}

#[cfg(tarpaulin)]
fn main() {}
