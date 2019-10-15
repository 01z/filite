#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde;

use crate::setup::Config;

use actix_web::{web, App, FromRequest, HttpServer};
use diesel::r2d2::{self, ConnectionManager};
use diesel::sqlite::SqliteConnection;
use std::process;

#[cfg(not(debug_assertions))]
use std::fs;

pub mod handlers;
pub mod models;
pub mod queries;
pub mod schema;
pub mod setup;

/// SQLite database connection pool
pub type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

/// Performs the initial setup
#[cfg(not(debug_assertions))]
fn init() -> Config {
    let data_dir = setup::get_data_dir();
    if !data_dir.exists() {
        eprintln!("Creating config file...");
        fs::create_dir_all(&data_dir)
            .unwrap_or_else(|e| eprintln!("Can't create config directory: {}.", e));
        Config::default().write_file().unwrap_or_else(|e| {
            eprintln!("{}", e);
            process::exit(1);
        });
        eprintln!(
            "To get started, edit the config file at {:?} and restart.",
            &data_dir
        );
        process::exit(0);
    }

    Config::read_file().unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    })
}

fn main() {
    let config = {
        #[cfg(debug_assertions)]
        {
            Config::debug()
        }

        #[cfg(not(debug_assertions))]
        {
            init()
        }
    };
    setup::init_logger();

    let pool = setup::create_pool(&config.database_url, config.pool_size);

    let port = config.port;
    let max_filesize = (config.max_filesize as f64 * 1.37) as usize;

    HttpServer::new(move || {
        App::new()
            .data(config.clone())
            .data(pool.clone())
            .wrap(setup::logger_middleware())
            .service(web::resource("/config").route(web::get().to(handlers::get_config)))
            .service(web::resource("/f").route(web::get().to_async(handlers::files::gets)))
            .service(web::resource("/l").route(web::get().to_async(handlers::links::gets)))
            .service(web::resource("/t").route(web::get().to_async(handlers::texts::gets)))
            .route("/f/{id}", web::get().to_async(handlers::files::get))
            .route("/l/{id}", web::get().to_async(handlers::links::get))
            .route("/t/{id}", web::get().to_async(handlers::texts::get))
            .service(
                web::resource("/f/{id}")
                    .data(web::Json::<handlers::files::PutFile>::configure(|cfg| {
                        cfg.limit(max_filesize)
                    }))
                    .route(web::put().to_async(handlers::files::put))
                    .route(web::delete().to_async(handlers::files::delete)),
            )
            .service(
                web::resource("/l/{id}")
                    .route(web::put().to_async(handlers::links::put))
                    .route(web::delete().to_async(handlers::links::delete)),
            )
            .service(
                web::resource("/t/{id}")
                    .route(web::put().to_async(handlers::texts::put))
                    .route(web::delete().to_async(handlers::texts::delete)),
            )
    })
    .bind(&format!("localhost:{}", port))
    .unwrap_or_else(|e| {
        eprintln!("Can't bind webserver to specified port: {}.", e);
        process::exit(1);
    })
    .run()
    .unwrap_or_else(|e| {
        eprintln!("Can't start webserver: {}.", e);
        process::exit(1);
    });
}
