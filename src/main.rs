#![allow(
	clippy::module_name_repetitions,
	clippy::unused_self,
	clippy::missing_const_for_fn, // TODO: Remove when #![feature(const_precise_live_drops)] gets stabilized
	clippy::struct_excessive_bools
)]

use std::collections::BTreeMap;

use chrono::SubsecRound;
use clap::StructOpt;
use url::Url;

use crate::beacon::Beacon;
use crate::framework::Framework;
use crate::model::Model;
use crate::output::BeaconOutput;

mod beacon;
mod endpoint;
mod error;
mod framework;
mod interface;
mod model;
mod output;
mod utils;

pub type Json = serde_json::Value;

#[derive(clap::Parser)]
#[clap(about, version, author)]
struct Args {
	/// Sets the level of verbosity
	#[clap(short, long, conflicts_with("quiet"))]
	verbose: bool,

	/// Do not print any logs
	#[clap(short, long, conflicts_with("summary"))]
	quiet: bool,

	/// Only log the summary of the results, do not output anything
	#[clap(short, long, conflicts_with("verbose"))]
	summary: bool,

	/// Only validate the framework referenced
	#[clap(long = "only-framework")]
	only_framework: bool,

	/// Location of the model
	#[clap(
		short,
		long,
		default_value = "https://github.com/MrRobb/beacon-v2-Models/BEACON-V2-draft4-Model"
	)]
	model: Url,

	/// Location of the framework
	#[clap(short, long, default_value = "https://github.com/MrRobb/beacon-framework-v2")]
	framework: Url,

	/// Url to the Beacon implementation
	url: Url,
}

fn main() {
	// Get args
	let matches = Args::parse();

	// Verbose

	if matches.quiet || matches.summary {
		std::env::set_var("RUST_LOG", "info");
		pretty_env_logger::init();
		log::set_max_level(log::LevelFilter::Off);
	}
	else if matches.verbose {
		std::env::set_var("RUST_LOG", "debug");
		pretty_env_logger::init();
	}
	else {
		std::env::set_var("RUST_LOG", "info");
		pretty_env_logger::init();
	}

	// Load framework
	let framework_location = matches.framework;
	log::debug!("Loading framework from: {}", &framework_location);
	let framework = Framework::load(&framework_location).expect("Loading framework failed");
	log::debug!("Framework loaded");

	// Load model
	let model = if matches.only_framework {
		None
	}
	else {
		let model_location = matches.model;
		log::debug!("Loading model from: {}", model_location);
		let model = Model::load(&model_location).expect("Loading model failed");
		log::info!("Number of entities of the model: {}", model.entities.len());
		Some(model)
	};

	// Load beacon
	let beacon_url = matches.url;
	log::info!("Validating implementation on {}", beacon_url);

	let output = match Beacon::new(model, framework, &beacon_url) {
		Ok(beacon) => beacon.validate(),
		Err(e) => BeaconOutput {
			name: format!("Unknown Beacon ({})", e),
			url: beacon_url,
			last_updated: chrono::offset::Utc::now().naive_utc().round_subsecs(6),
			entities: BTreeMap::new(),
		},
	};

	if matches.summary {
		log::set_max_level(log::LevelFilter::Trace);
		output.summary();
	}
	else {
		if !matches.quiet {
			eprintln!();
		}
		output.summary();
		let payload = serde_json::to_string_pretty(&output).unwrap();
		println!("{}", payload);
	}
}
