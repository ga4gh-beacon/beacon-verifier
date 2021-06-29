use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;

use clap::crate_authors;
use clap::crate_version;
use clap::load_yaml;
use clap::App;
use clap::AppSettings;
use serde_json::Value;

use crate::model::Model;

mod config;
mod model;
mod utils;

fn should_replace_key_value(k: &str, v: &mut Value, base_url: &str) {
	if k.starts_with('$') {
		if let Value::String(path) = v {
			if Path::new(path).extension().is_some() && Path::new(path).extension().unwrap() == "json" {
				if let Err(e) = url::Url::parse(&path.clone()) {
					if e == url::ParseError::RelativeUrlWithoutBase {
						*path = utils::normalize_path(Path::new(&format!("{}/{}", base_url, &path)))
							.to_string_lossy()
							.to_string();
						// paris::info!("PATH: {}", path);
					}
				}
			}
		}
	}
}

fn deep_keys(value: &mut Value, base_url: &str) {
	match value {
		Value::Object(map) => {
			for (k, v) in map {
				should_replace_key_value(k, v, base_url);
				deep_keys(v, base_url);
			}
		}
		Value::Array(array) => {
			for v in array.iter_mut() {
				deep_keys(v, base_url);
			}
		}
		_ => (),
	}
}

fn run() -> Result<(), Box<dyn Error>> {
	// Get args
	let yaml = load_yaml!("../cli.yaml");
	let matches = App::from(yaml)
		.version(crate_version!())
		.author(crate_authors!())
		.global_setting(AppSettings::ArgRequiredElseHelp)
		.global_setting(AppSettings::ColorAlways)
		.global_setting(AppSettings::ColoredHelp)
		.get_matches();

	// Coger el directorio base
	let dir = Path::new(matches.value_of("path").unwrap()).canonicalize().unwrap();
	paris::info!("PATH: {}", dir.display());

	// Create model
	let mut model = Model::new();

	// Encontrar todos los subdirectorios
	for entry in walkdir::WalkDir::new(dir).into_iter().flatten() {
		if entry.path().extension() == Some(OsStr::new("json")) {
			model.add(entry.path())?;
		}
	}

	model.validate();

	Ok(())
}

fn main() {
	if let Err(err) = run() {
		paris::error!("{}", err);
		std::process::exit(1);
	}
}
