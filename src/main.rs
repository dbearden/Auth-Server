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
const CONFIG_RE: &str = "regex";
const CONFIG_DB_HOST: &str = "host";
const CONFIG_DB_DBNAME: &str = "dbname";
const CONFIG_DB_USER: &str = "user";
const CONFIG_DB_PASSWORD: &str = "password";
const CONFIG_SOCKET: &str = "socket";

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
            .get_str(CONFIG_RE)
            .expect("Could not retrieve password regex.")
    )
    .expect("Could not create password regex.");
}

fn get_pool() -> DbPool {
    let mut pool_config = Config::new();
    pool_config.host = SETTINGS.get_str(CONFIG_DB_HOST).ok();
    pool_config.dbname = SETTINGS.get_str(CONFIG_DB_DBNAME).ok();
    pool_config.user = SETTINGS.get_str(CONFIG_DB_USER).ok();
    pool_config.password = SETTINGS.get_str(CONFIG_DB_PASSWORD).ok();
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
    .bind(SETTINGS.get_str(CONFIG_SOCKET).unwrap())?
    .run()
    .await
}
