use std::sync::Arc;

use futures_util::future::join;
use gumdrop::Options;
use repo::Repo;
use tokio_schedule::{every, Job};
use typed_sled::Tree;

mod codecs;
mod error;
mod repo;
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

    let db = sled::open(&opts.db_path).expect("failed to open the db");

    let repo = Repo {
        subs: Tree::open(&db, "subs"),
        unread: Tree::open(&db, "unread"),
        starred: Tree::open(&db, "starred"),
        entries: Tree::open(&db, "entries"),
        db,
    };
    let repo = Arc::new(repo);

    let scheduler = every(opts.interval_minutes)
        .minutes()
        .perform(|| repo::trigger_refresh_all_subsripions(repo.clone()));
    let service = service::run(repo.clone(), &opts);

    join(scheduler, service).await;
}
