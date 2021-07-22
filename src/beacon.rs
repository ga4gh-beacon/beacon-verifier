use std::error::Error;
use std::path::{Path, PathBuf};

use url::Url;

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
	pub fn new(spec: Spec, framework: Framework, url: Url) -> Result<Self, Box<dyn Error>> {
		let mut info_url = url.clone();
		info_url.set_path(Path::new(url.path()).join("info").to_str().unwrap_or(""));
		let info: Json = ureq::get(&info_url.to_string()).call()?.into_json()?;
		log::trace!("{}", info);

		let name_json = if let Some(response) = info.get("response") {
			if let Some(name) = response.get("name") {
				name.clone()
			}
			else {
				log::error!("Please look at https://github.com/ga4gh-beacon/beacon-framework-v2/blob/main/responses/sections/beaconInfoResults.json");
				log::error!("No 'name' in {}/info inside json object 'response'", url);
				Json::String("Unknown name".into())
			}
		}
		else {
			log::error!("Please look at https://github.com/ga4gh-beacon/beacon-framework-v2/blob/main/responses/beaconInfoResponse.json");
			log::error!("No property 'response' in {}/info", url);
			Json::String("Unknown name".into())
		};

		let name = if name_json.is_string() {
			name_json.as_str().unwrap().to_string()
		}
		else {
			name_json.to_string()
		};

		Ok(Self {
			name,
			url,
			spec,
			framework,
		})
	}

	fn valid_schema(&self, schema: &Json, instance: &Json) -> Option<bool> {
		let json_schema = match jsonschema::JSONSchema::options().with_meta_schemas().compile(schema) {
			Ok(schema) => schema,
			Err(e) => {
				log::error!("{:?}", e);
				return None;
			},
		};

		let valid = match json_schema.validate(instance) {
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
		};

		Some(valid)
	}

	fn valid_endpoint(&self, entity: &Entity, endpoint_url: &Url) -> Option<bool> {
		// Query endpoint
		let response = match ureq::get(endpoint_url.as_str()).call() {
			Ok(response) => response,
			Err(e) => {
				log::error!("{:?}", e);
				return None;
			},
		};

		let response_json = match response.into_json::<Json>() {
			Ok(response_json) => response_json,
			Err(e) => {
				log::error!("{:?}", e);
				return Some(false);
			},
		};

		let schema = match jsonschema::JSONSchema::options()
			.with_meta_schemas()
			.compile(&entity.schema)
		{
			Ok(schema) => schema,
			Err(e) => {
				log::error!("{:?}", e);
				return None;
			},
		};

		log::debug!("Validating...");
		let resp = if let Some(r) = response_json["resultSets"].as_object() {
			r
		}
		else {
			log::error!("No resultSets in response");
			return Some(false);
		};

		if resp["exists"] == "false" {
			return Some(true);
		}

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

		Some(valid)
	}

	pub fn validate(self) -> Result<BeaconOutput, Box<dyn Error>> {
		let mut output = Vec::new();

		// Validate configuration
		eprintln!();
		let mut configuration_url = self.url.clone();
		configuration_url.set_path(Path::new(self.url.path()).join("configuration").to_str().unwrap_or(""));
		let valid = match utils::ping_url(&configuration_url) {
			Ok(configuration_json) => self.valid_schema(&self.framework.configuration_json, &configuration_json),
			Err(e) => {
				log::error!("{}", e);
				None
			},
		};
		output.push(EndpointOutput {
			name: "Configuration".into(),
			url: configuration_url,
			valid,
		});

		// Validate beacon map
		eprintln!();
		let mut beacon_map_url = self.url.clone();
		beacon_map_url.set_path(Path::new(self.url.path()).join("map").to_str().unwrap_or(""));
		let valid = match utils::ping_url(&beacon_map_url) {
			Ok(beacon_map_json) => self.valid_schema(&self.framework.beacon_map_json, &beacon_map_json),
			Err(e) => {
				log::error!("{}", e);
				None
			},
		};
		output.push(EndpointOutput {
			name: "BeaconMap".into(),
			url: beacon_map_url,
			valid,
		});

		// Validate entry types
		eprintln!();
		let mut entry_types_url = self.url.clone();
		entry_types_url.set_path(Path::new(self.url.path()).join("entry_types").to_str().unwrap_or(""));
		let valid = match utils::ping_url(&entry_types_url) {
			Ok(entry_types_json) => self.valid_schema(&self.framework.entry_types_json, &entry_types_json),
			Err(e) => {
				log::error!("{}", e);
				None
			},
		};
		output.push(EndpointOutput {
			name: "EntryTypes".into(),
			url: entry_types_url,
			valid,
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

			let valid = self.valid_endpoint(entity, &replaced_url);

			output.push(EndpointOutput {
				name: entity.name.clone(),
				url: replaced_url.clone(),
				valid,
			})
		}

		Ok(BeaconOutput {
			name: self.name,
			url: self.url,
			entities: output,
		})
	}
}
