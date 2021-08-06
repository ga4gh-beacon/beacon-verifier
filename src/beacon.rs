use std::path::{Path, PathBuf};

use url::Url;

use crate::error::VerifierError;
use crate::framework::Framework;
use crate::interface::{BeaconOutput, EndpointOutput};
use crate::spec::{Entity, Spec};
use crate::{utils, Json};

pub struct Beacon {
	name: String,
	url: Url,
	spec: Spec,
	framework: Framework,
}

impl Beacon {
	pub fn new(spec: Spec, framework: Framework, url: Url) -> Self {
		let mut info_url = url.clone();
		info_url.set_path(Path::new(url.path()).join("info").to_str().unwrap_or(""));
		let info: Json = ureq::get(&info_url.to_string()).call().unwrap().into_json().unwrap();
		log::trace!("{}", info);

		Self {
			name: Self::get_name(&info, &url),
			url,
			spec,
			framework,
		}
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

	fn valid_schema(&self, schema: &Json, instance: &Json) -> (Option<bool>, Option<VerifierError>) {
		let json_schema = match jsonschema::JSONSchema::options().with_meta_schemas().compile(schema) {
			Ok(schema) => schema,
			Err(e) => {
				log::error!("{:?}", e);
				return (None, Some(VerifierError::BadSchema));
			},
		};

		let (valid, error) = match json_schema.validate(instance) {
			Ok(_) => {
				log::info!("VALID");
				(true, None)
			},
			Err(errors) => {
				log::error!("NOT VALID:");
				let mut er = String::new();
				for e in errors {
					log::error!(
						"   ERROR: {} on property path {}",
						e.to_string(),
						e.instance_path.to_string(),
					);
					er.push_str(&e.to_string());
					er.push('\n');
				}
				(false, Some(VerifierError::BadResponse(er)))
			},
		};

		(Some(valid), error)
	}

	fn valid_endpoint(&self, entity: &Entity, endpoint_url: &Url) -> (Option<bool>, Option<VerifierError>) {
		// Query endpoint
		let response = match ureq::get(endpoint_url.as_str()).call() {
			Ok(response) => response,
			Err(e) => {
				log::error!("{:?}", e);
				return (None, Some(VerifierError::UnresponsiveEndpoint(endpoint_url.clone())));
			},
		};

		let response_json = match response.into_json::<Json>() {
			Ok(response_json) => response_json,
			Err(e) => {
				log::error!("{:?}", e);
				return (Some(false), Some(VerifierError::ResponseIsNotJson));
			},
		};

		let schema = match jsonschema::JSONSchema::options()
			.with_meta_schemas()
			.compile(&entity.schema)
		{
			Ok(schema) => schema,
			Err(e) => {
				log::error!("{:?}", e);
				return (None, Some(VerifierError::BadSchema));
			},
		};

		log::debug!("Validating...");
		let resp = if let Some(r) = response_json["resultSets"].as_object() {
			r
		}
		else {
			log::error!("No resultSets in response");
			return (Some(false), Some(VerifierError::NoResultSets));
		};

		if resp["exists"] == "false" {
			return (Some(true), None);
		}

		let error = None;
		let valid = resp["results"]
			.as_array()
			.unwrap()
			.iter()
			.all(|result| match schema.validate(result) {
				Ok(_) => {
					log::info!("VALID");
					true
				},
				Err(errors) => {
					log::error!("NOT VALID:");
					for e in errors {
						log::error!(
							"   ERROR: {} on property path {}",
							e.to_string(),
							e.instance_path.to_string(),
						);
					}
					false
				},
			});

		(Some(valid), error)
	}

	pub fn validate(self) -> BeaconOutput {
		let mut output = Vec::new();

		// Validate configuration
		eprintln!();
		let mut configuration_url = self.url.clone();
		configuration_url.set_path(Path::new(self.url.path()).join("configuration").to_str().unwrap_or(""));
		let (valid, error) = match utils::ping_url(&configuration_url) {
			Ok(configuration_json) => self.valid_schema(&self.framework.configuration_json, &configuration_json),
			Err(e) => {
				log::error!("{}", e);
				(None, Some(e))
			},
		};
		output.push(EndpointOutput {
			name: "Configuration".into(),
			url: configuration_url,
			valid,
			error: error.map(|e| e.to_string())
		});

		// Validate beacon map
		eprintln!();
		let mut beacon_map_url = self.url.clone();
		beacon_map_url.set_path(Path::new(self.url.path()).join("map").to_str().unwrap_or(""));
		let (valid, error) = match utils::ping_url(&beacon_map_url) {
			Ok(beacon_map_json) => self.valid_schema(&self.framework.beacon_map_json, &beacon_map_json),
			Err(e) => {
				log::error!("{}", e);
				(None, Some(e))
			},
		};
		output.push(EndpointOutput {
			name: "BeaconMap".into(),
			url: beacon_map_url,
			valid,
			error: error.map(|e| e.to_string())
		});

		// Validate entry types
		eprintln!();
		let mut entry_types_url = self.url.clone();
		entry_types_url.set_path(Path::new(self.url.path()).join("entry_types").to_str().unwrap_or(""));
		let (valid, error) = match utils::ping_url(&entry_types_url) {
			Ok(entry_types_json) => self.valid_schema(&self.framework.entry_types_json, &entry_types_json),
			Err(e) => {
				log::error!("{}", e);
				(None, Some(e))
			},
		};
		output.push(EndpointOutput {
			name: "EntryTypes".into(),
			url: entry_types_url,
			valid,
			error: error.map(|e| e.to_string())
		});

		// Validate endpoints configuration
		// TODO: Validate OpenAPI 3.0

		// Validate entities
		for entity in &self.spec.entities {
			// Get params
			eprintln!();
			log::info!("Validating {:?}", entity.name);
			let mut replaced_url = self.url.clone();
			let new_path: PathBuf = PathBuf::from(replaced_url.path())
				.components()
				.chain(Path::new(entity.url.path()).components().skip(1))
				.collect();
			replaced_url.set_path(new_path.to_str().unwrap_or(""));
			log::debug!("GET {}", replaced_url);

			let (valid, error) = self.valid_endpoint(entity, &replaced_url);

			output.push(EndpointOutput {
				name: entity.name.clone(),
				url: replaced_url.clone(),
				valid,
				error: error.map(|e| e.to_string()),
			});
		}

		BeaconOutput {
			name: self.name,
			url: self.url,
			entities: output,
		}
	}
}
