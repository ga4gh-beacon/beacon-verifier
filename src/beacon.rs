use std::path::Path;

use chrono::SubsecRound;
use url::Url;

use crate::endpoint::BeaconEndpoint;
use crate::error::VerifierError;
use crate::framework::Framework;
use crate::interface::Granularity;
use crate::output::{BeaconOutput, EndpointReport, Output};
use crate::spec::{Entity, Spec};
use crate::{utils, Json};

pub struct Beacon {
	name: String,
	url: Url,
	spec: Spec,
	framework: Framework,
}

impl Beacon {
	pub fn new(spec: Spec, framework: Framework, url: &Url) -> Result<Self, VerifierError> {
		let mut info_url = url.clone();
		info_url.set_path(Path::new(url.path()).join("info").to_str().unwrap_or(""));
		let info: Json = reqwest::blocking::get(&info_url.to_string())?.json().unwrap();
		log::trace!("{}", info);

		Ok(Self {
			name: Self::get_name(&info, url),
			url: url.clone(),
			spec,
			framework,
		})
	}

	fn get_name(info: &Json, url: &Url) -> String {
		let name_json = if let Some(response) = info.get("response") {
			if let Some(name) = response.get("name") {
				name.clone()
			}
			else {
				log::error!(
					"{}",
					VerifierError::BadInfo(format!("No 'name' in {}/info inside json object 'response'", url))
				);
				Json::String("Unknown name (bad /info)".into())
			}
		}
		else {
			log::error!(
				"{}",
				VerifierError::BadInfo(format!("No 'response' property in {}/info", url))
			);
			Json::String("Unknown name (bad /info)".into())
		};

		let name = if name_json.is_string() {
			name_json.as_str().unwrap().to_string()
		}
		else {
			name_json.to_string()
		};

		name
	}

	fn valid_endpoint(&self, entity: &Entity, endpoint_url: &Url) -> EndpointReport {
		// Get response
		let response_json = match utils::ping_url(endpoint_url) {
			Ok(j) => j,
			Err(e) => {
				return EndpointReport::new().null(e);
			},
		};

		// Test granularity
		let granularity: Result<Granularity, VerifierError> = response_json
			.as_object()
			.expect("JSON is not an object")
			.get("meta")
			.expect("No 'meta' property was found")
			.as_object()
			.expect("'meta' is not an object")
			.get("returnedGranularity")
			.expect("No 'returnedGranularity' property was found")
			.as_str()
			.expect("'returnedGranularity' is not a string")
			.try_into();

		// Test response
		match granularity {
			Ok(g) => {
				let valid_against_framework = match g {
					Granularity::Boolean => {
						BeaconEndpoint::validate_against_framework(&response_json, &self.framework.boolean_json)
					},
					Granularity::Count => {
						BeaconEndpoint::validate_against_framework(&response_json, &self.framework.count_json)
					},
					Granularity::Aggregated | Granularity::Record => {
						BeaconEndpoint::validate_against_framework(&response_json, &self.framework.result_sets_json)
					},
				};
				if let Err(e) = valid_against_framework {
					return EndpointReport::new().error(e);
				}

				if Granularity::Record == g {
					// Compile entity schema
					let schema = match jsonschema::JSONSchema::options()
						.with_meta_schemas()
						.compile(&entity.schema)
					{
						Ok(schema) => schema,
						Err(e) => {
							log::error!("{:?}", e);
							return EndpointReport::new().null(VerifierError::BadSchema);
						},
					};
					BeaconEndpoint::validate_resultset_response(&response_json, &schema)
				}
				else {
					EndpointReport::new().ok(Some(response_json))
				}
			},
			Err(e) => EndpointReport::new().error(e),
		}
	}

	fn validate_against_framework(&self, location: &str, schema: &Json) -> EndpointReport {
		let mut url = self.url.clone();
		url.set_path(Path::new(self.url.path()).join(&location).to_str().unwrap_or(""));
		let report = match utils::ping_url(&url) {
			Ok(beacon_map_json) => {
				let json_schema = match jsonschema::JSONSchema::options().with_meta_schemas().compile(schema) {
					Ok(schema) => schema,
					Err(e) => {
						log::error!("{:?}", e);
						return EndpointReport::new().null(VerifierError::BadSchema);
					},
				};
				match utils::valid_schema(&json_schema, &beacon_map_json) {
					Ok(output) => EndpointReport::new().ok(Some(output)),
					Err(e) => EndpointReport::new().error(e),
				}
			},
			Err(e) => {
				log::error!("{}", e);
				EndpointReport::new().null(e)
			},
		};
		report.url(url)
	}

	pub fn validate(self) -> BeaconOutput {
		let mut output = Output::new();

		// Validate configuration
		let report = self.validate_against_framework("configuration", &self.framework.configuration_json);
		output.push("Configuration".into(), report.name("Configuration"));

		// Validate beacon map
		eprintln!();
		let report = self.validate_against_framework("map", &self.framework.beacon_map_json);
		output.push("BeaconMap".into(), report.name("BeaconMap"));

		// Validate entry types
		eprintln!();
		let report = self.validate_against_framework("entry_types", &self.framework.entry_types_json);
		output.push("EntryTypes".into(), report.name("EntryTypes"));

		// Validate endpoints configuration
		// TODO: Validate OpenAPI 3.0

		// Validate entities
		for entity in &self.spec.entities {
			// Get params
			eprintln!();
			log::info!("Validating {:?}", entity.name);
			let replaced_url = utils::url_join(&self.url, &entity.url);
			log::debug!("GET {}", replaced_url);

			// Validate /endpoint
			let report = self.valid_endpoint(entity, &replaced_url);
			let ids = utils::get_ids(&report);
			output.push(
				entity.name.clone(),
				report
					.name(&format!("{} all entries", entity.name.clone()))
					.url(replaced_url.clone()),
			);

			// Validate /endpoint/{id}
			if let Some(url_single) = &entity.url_single {
				let mut replaced_url_single = utils::url_join(&self.url, url_single);
				replaced_url_single = utils::replace_vars(
					&replaced_url_single,
					vec![("id", &ids.clone().unwrap_or_else(|| String::from("_id_")))],
				);
				let report_ids = if ids.is_none() {
					EndpointReport::new().null(VerifierError::NoIds)
				}
				else {
					self.valid_endpoint(entity, &replaced_url_single)
				};
				output.push(
					entity.name.clone(),
					report_ids
						.name(&format!("{} single entry", entity.name.clone()))
						.url(replaced_url_single.clone()),
				);
			}

			// Validate /endpoint?filtering_term=value
			if let Some(filtering_terms_url) = &entity.filtering_terms_url {
				let available_filtering_terms = utils::get_filtering_terms(filtering_terms_url);
				for filtering_term in available_filtering_terms {
					let replaced_url_filter = utils::url_join(&self.url, &filtering_term.url);
					let report = self.valid_endpoint(entity, &replaced_url_filter);
					output.push(
						entity.name.clone(),
						report
							.name(&format!("{} filtering terms", entity.name.clone()))
							.url(replaced_url_filter.clone()),
					);
				}
			}

			// Validate /endpoint/{id}/endpoint
			if let Some(related_endpoints) = &entity.related_endpoints {
				for (_, related_enpoint) in related_endpoints.iter() {
					let mut replaced_url_related = utils::url_join(&self.url, &related_enpoint.url);
					replaced_url_related = utils::replace_vars(
						&replaced_url_related,
						vec![("id", &ids.clone().unwrap_or_else(|| String::from("_id_")))],
					);
					let report_ids = if ids.is_none() {
						EndpointReport::new().null(VerifierError::NoIds)
					}
					else {
						replaced_url_related = utils::replace_vars(
							&replaced_url_related,
							vec![("id", &ids.clone().unwrap_or_else(|| String::from("_id_")))],
						);
						self.valid_endpoint(entity, &replaced_url_related)
					};
					output.push(
						entity.name.clone(),
						report_ids
							.name(&format!(
								"{} related with a {}",
								self.spec
									.entities_names
									.get(&related_enpoint.returned_entry_type)
									.unwrap_or(&String::from("Unknown entity")),
								entity.name.clone()
							))
							.url(replaced_url_related.clone()),
					);
				}
			}
		}

		BeaconOutput {
			name: self.name,
			url: self.url,
			last_updated: chrono::offset::Utc::now().naive_utc().round_subsecs(6),
			entities: output.finish(),
		}
	}
}
