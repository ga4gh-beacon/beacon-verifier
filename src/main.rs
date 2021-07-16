#![allow(clippy::module_name_repetitions, clippy::unused_self)]

use std::error::Error;

use clap::{crate_authors, crate_version, load_yaml, App, AppSettings};
use url::Url;

use crate::beacon::Beacon;
use crate::framework::Framework;
use crate::spec::Spec;

mod beacon;
mod framework;
mod interface;
mod spec;
mod utils;

pub type Json = serde_json::Value;

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

	// Verbose
	if matches.is_present("verbose") {
		std::env::set_var("RUST_LOG", "debug");
	}
	else {
		std::env::set_var("RUST_LOG", "info");
	}
	pretty_env_logger::init();

	// Load framework
	let framework_location = matches.value_of_t("framework")?;
	log::debug!("Loading framework from: {}", &framework_location);
	let framework = Framework::load(&framework_location).expect("Loading framework failed");
	log::debug!("Framework loaded");

	// Load spec
	let spec_location = matches.value_of_t("spec")?;
	log::debug!("Loading spec from: {}", spec_location);
	let spec = Spec::load(&spec_location).expect("Loading spec failed");
	let n_entitites = spec.validate(&framework)?;
	log::info!("Valid spec (number of entities: {})", n_entitites);

	// Validate beacons
	if !matches.is_present("only-spec") {
		// Load beacons
		let mut output = Vec::new();
		for beacon_url in matches.values_of_t::<Url>("URLS")? {
			log::info!("Validating implementation on {}", beacon_url);
			match Beacon::new(spec.clone(), framework.clone(), beacon_url) {
				Ok(beacon) => match beacon.validate() {
					Ok(beacon_output) => output.push(beacon_output),
					Err(e) => {
						log::error!("{:?}", e);
					},
				},
				Err(e) => {
					log::error!("{:?}", e);
				},
			}
		}

		let payload = serde_json::to_string_pretty(&output)?;
		println!("{}", payload);
	}

	Ok(())
}

fn main() {
	if let Err(err) = run() {
		let _ = pretty_env_logger::try_init();
		log::error!("{}", err);
		std::process::exit(1);
	}
}
