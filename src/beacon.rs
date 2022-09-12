use std::path::Path;

use chrono::SubsecRound;
use url::Url;

use crate::error::VerifierError;
use crate::framework::Framework;
use crate::model::Model;
use crate::output::{BeaconOutput, EndpointReport, Output};
use crate::{utils, Json};

use clap::Parser;

pub struct Beacon {
	name: String,
	url: Url,
	model: Option<Model>,
	framework: Framework,
}

impl Beacon {
	pub fn new(model: Option<Model>, framework: Framework, url: &Url) -> Result<Self, VerifierError> {
		let mut info_url = url.clone();
		info_url.set_path(Path::new(url.path()).join("info").to_str().unwrap_or(""));

                // switch to builder to allow ignore self-signed ssl certs
                let matches = crate::Args::parse();
                let client = reqwest::blocking::Client::builder()
                    .danger_accept_invalid_certs(matches.ssl_no_verify)
                    .build()?;
		let info: Json = client.get(&info_url.to_string()).send()?.json().unwrap();
		log::trace!("{}", info);

		Ok(Self {
			name: Self::get_name(&info, url),
			url: url.clone(),
			model,
			framework,
		})
	}

	fn get_name(info: &Json, url: &Url) -> String {
		let name_json = info.get("response").map_or_else(
			|| {
				log::error!(
					"{}",
					VerifierError::BadInfo(format!("No 'response' property in {}/info", url))
				);
				Json::String("Unknown name (bad /info)".into())
			},
			|response| {
				response.get("name").map_or_else(
					|| {
						log::error!(
							"{}",
							VerifierError::BadInfo(format!("No 'name' in {}/info inside json object 'response'", url))
						);
						Json::String("Unknown name (bad /info)".into())
					},
					std::clone::Clone::clone,
				)
			},
		);

		let name = if name_json.is_string() {
			name_json.as_str().unwrap().to_string()
		}
		else {
			name_json.to_string()
		};

		name
	}

	fn validate_against_framework(&self, entity_name: &str, location: &str, schema: &Json) -> EndpointReport {
		let mut url = self.url.clone();
		url.set_path(Path::new(self.url.path()).join(&location).to_str().unwrap_or(""));
		let report = match utils::ping_url(&url) {
			Ok(beacon_map_json) => {
				let json_schema = match jsonschema::JSONSchema::options().with_meta_schemas().compile(schema) {
					Ok(schema) => schema,
					Err(e) => {
						log::error!("{:?}", e);
						return EndpointReport::new(entity_name, &self.name, self.url.clone())
							.null(VerifierError::BadSchema);
					},
				};
				match utils::valid_schema(&json_schema, &beacon_map_json) {
					Ok(output) => EndpointReport::new(entity_name, &self.name, self.url.clone()).ok(Some(output)),
					Err(e) => EndpointReport::new(entity_name, &self.name, self.url.clone()).error(e),
				}
			},
			Err(e) => {
				log::error!("{}", e);
				EndpointReport::new(entity_name, &self.name, self.url.clone()).null(e)
			},
		};
		report.url(url)
	}

	pub fn validate(self) -> BeaconOutput {
		let mut output = Output::new();

		// Validate info
		log::info!("Validating \"Info\"");
		let report = self.validate_against_framework("Info", "info", &self.framework.info_json);
		output.push(report);

		// Validate configuration
		log::info!("Validating \"Configuration\"");
		let report =
			self.validate_against_framework("Configuration", "configuration", &self.framework.configuration_json);
		output.push(report);

		// Validate beacon map
		log::info!("Validating \"Beacon Map\"");
		let report = self.validate_against_framework("BeaconMap", "map", &self.framework.beacon_map_json);
		output.push(report);

		// Validate entry types
		log::info!("Validating \"Entry Types\"");
		let report = self.validate_against_framework("EntryTypes", "entry_types", &self.framework.entry_types_json);
		output.push(report);

		// Validate endpoints configuration
		// TODO: Validate OpenAPI 3.0

		// Validate entities
		if let Some(model) = self.model {
			let boolean_json = utils::compile_schema(&self.framework.boolean_json);
			let count_json = utils::compile_schema(&self.framework.count_json);
			let result_sets_json = utils::compile_schema(&self.framework.result_sets_json);
			let collections_json = utils::compile_schema(&self.framework.collections_json);
			model
				.endpoints(&self.url)
				.into_iter()
				.map(|endpoint| {
					log::info!("Validating {:?}", endpoint.name);
					endpoint.validate(
						&self.url,
						&boolean_json,
						&count_json,
						&result_sets_json,
						&collections_json,
					)
				})
				.for_each(|report| output.push(report));
		}

		BeaconOutput {
			name: self.name,
			url: self.url,
			last_updated: chrono::offset::Utc::now().naive_utc().round_subsecs(6),
			entities: output.finish(),
		}
	}
}
