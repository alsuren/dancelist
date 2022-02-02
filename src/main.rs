// Copyright 2022 the dancelist authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod config;
mod controllers;
mod errors;
mod model;

use crate::{
    config::Config,
    controllers::{bands, callers, index, organisations},
    errors::internal_error,
    model::events::Events,
};
use axum::{
    routing::{get, get_service},
    AddExtensionLayer, Router,
};
use eyre::Report;
use log::info;
use schemars::schema_for;
use std::{env, path::Path, process::exit};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> Result<(), Report> {
    stable_eyre::install()?;
    pretty_env_logger::init();
    color_backtrace::install();

    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        serve().await
    } else if args.len() == 2 && args[1] == "schema" {
        // Output JSON schema for events.
        print!("{}", event_schema()?);
        Ok(())
    } else if args.len() >= 2 && args.len() <= 3 && args[1] == "validate" {
        validate(args.get(2).map(Path::new))
    } else {
        eprintln!("Invalid command.");
        exit(1);
    }
}

fn validate(path: Option<&Path>) -> Result<(), Report> {
    let events = if let Some(path) = path {
        if path.is_dir() {
            Events::load_directory(path)?
        } else {
            Events::load_file(path)?
        }
    } else {
        let config = Config::from_file()?;
        Events::load_directory(&config.events_dir)?
    };
    println!("Successfully validated {} events.", events.events.len());

    Ok(())
}

async fn serve() -> Result<(), Report> {
    let config = Config::from_file()?;
    let events = Events::load_directory(&config.events_dir)?;

    let app = Router::new()
        .route("/", get(index::index))
        .route("/index.toml", get(index::index_toml))
        .route("/index.yaml", get(index::index_yaml))
        .route("/balfolk", get(index::balfolk))
        .route("/bands", get(bands::bands))
        .route("/callers", get(callers::callers))
        .route("/organisations", get(organisations::organisations))
        .nest(
            "/stylesheets",
            get_service(ServeDir::new(config.public_dir.join("stylesheets")))
                .handle_error(internal_error),
        )
        .layer(AddExtensionLayer::new(events));

    info!("Listening on {}", config.bind_address);
    axum::Server::bind(&config.bind_address)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

/// Returns the JSON schema for events.
fn event_schema() -> Result<String, Report> {
    let schema = schema_for!(Events);
    Ok(serde_json::to_string_pretty(&schema)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read_to_string;

    #[test]
    fn json_schema_matches() {
        assert_eq!(
            event_schema().unwrap(),
            read_to_string("events_schema.json").unwrap()
        );
    }
}
