use std::sync::Arc;
use std::time::Duration;

use futures_util::future::join;
use gumdrop::Options;
use refresh::refresh_all_feeds;
use repo::Repo;

mod codecs;
mod refresh;
mod repo;
mod result;
mod service;
mod types;

#[derive(Debug, Clone, Options)]
pub struct AppConfig {
    #[options(help = "print help message")]
    help: bool,
    #[options(help = "port to bind by the service", default = "4000")]
    port: u16,
    #[options(help = "directory to store the database in", default = "db")]
    db_path: String,
    #[options(help = "basic auth password", required)]
    user: String,
    #[options(help = "basic auth user name", required)]
    password: String,
    #[options(help = "refresh time interval in minutes", default = "30")]
    interval_minutes: u32,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let opts = AppConfig::parse_args_default_or_exit();

    let repo = Arc::new(Repo::new(&opts.db_path).unwrap());
    let daemon = tokio::spawn(refresh_daemon(repo.clone(), opts.interval_minutes.into()));
    let service = service::run(repo, &opts);

    join(daemon, service).await.0.unwrap();
}

async fn refresh_daemon(repo: Arc<Repo>, interval: u64) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval * 60));

    loop {
        interval.tick().await;
        if let Err(err) = refresh_all_feeds(&repo).await {
            tracing::error!("subscription refresh failed: {err}");
        }
    }
}
