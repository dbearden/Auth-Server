use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer};
use deadpool::managed::PoolConfig;
use deadpool_postgres::{Config, ManagerConfig, RecyclingMethod};
use env_logger::Env;
use lazy_static::lazy_static;
use regex::Regex;
use tokio_postgres::NoTls;

mod handlers;
mod models;
use handlers::{login, register};

const POOL_SIZE: usize = 5;

type DbPool = deadpool_postgres::Pool;

lazy_static! {
    static ref SETTINGS: config::Config = {
        let mut config = config::Config::default();
        config
            .merge(config::File::with_name("config/settings.toml"))
            .expect("Issue loading settings.toml");
        config
    };
    static ref REGEX: Regex = Regex::new(
        &SETTINGS
            .get_str("regex")
            .expect("Could not retrieve password regex.")
    )
    .expect("Could not create password regex.");
}

fn get_pool() -> DbPool {
    let mut pool_config = Config::new();
    pool_config.host = SETTINGS.get_str("host").ok();
    pool_config.dbname = SETTINGS.get_str("dbname").ok();
    pool_config.user = SETTINGS.get_str("user").ok();
    pool_config.password = SETTINGS.get_str("password").ok();
    pool_config.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    pool_config.pool = Some(PoolConfig::new(POOL_SIZE));
    pool_config.create_pool(NoTls).unwrap()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::from_env(Env::default().default_filter_or("info")).init();
    let pool = get_pool();
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .data(pool.clone())
            .service(login)
            .service(register)
    })
    .bind(SETTINGS.get_str("socket").unwrap())?
    .run()
    .await
}
