#![allow(clippy::module_name_repetitions, clippy::unused_self, clippy::missing_const_for_fn)]

use std::collections::BTreeMap;

use chrono::SubsecRound;
use clap::{crate_authors, crate_version, load_yaml, App, AppSettings};

use crate::beacon::Beacon;
use crate::framework::Framework;
use crate::output::BeaconOutput;
use crate::spec::Spec;

mod beacon;
mod error;
mod framework;
mod interface;
mod output;
mod spec;
mod utils;

pub type Json = serde_json::Value;

fn main() {
	// Get args
	let yaml = load_yaml!("../cli.yaml");
	let matches = App::from(yaml)
		.version(crate_version!())
		.author(crate_authors!())
		.color(clap::ColorChoice::Always)
		.global_setting(AppSettings::ArgRequiredElseHelp)
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
	let framework_location = matches.value_of_t("framework").unwrap();
	log::debug!("Loading framework from: {}", &framework_location);
	let framework = Framework::load(&framework_location).expect("Loading framework failed");
	log::debug!("Framework loaded");

	// Load spec
	let spec_location = matches.value_of_t("spec").unwrap();
	log::debug!("Loading spec from: {}", spec_location);
	let spec = Spec::load(&spec_location).expect("Loading spec failed");
	let n_entitites = spec.validate(&framework);
	log::info!("Valid spec (number of entities: {})", n_entitites);

	// Validate beacons
	if !matches.is_present("only-spec") {
		// Load beacon
		let beacon_url = matches.value_of_t("URL").expect("Invalid argument");
		log::info!("Validating implementation on {}", beacon_url);
		let output = match Beacon::new(spec, framework, &beacon_url) {
			Ok(beacon) => beacon.validate(),
			Err(e) => BeaconOutput {
				name: format!("Unknown Beacon ({})", e),
				url: beacon_url,
				last_updated: chrono::offset::Utc::now().naive_utc().round_subsecs(6),
				entities: BTreeMap::new(),
			},
		};

		let payload = serde_json::to_string_pretty(&output).unwrap();
		println!("{}", payload);
	}
}
