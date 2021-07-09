use std::error::Error;

use url::Url;

use crate::interface::{BeaconOutput, EntityOutput};
use crate::spec::{Entity, Spec};
use crate::Json;

pub struct Beacon {
	name: String,
	url: Url,
	spec: Spec,
}

impl Beacon {
	pub fn new(spec: Spec, url: Url) -> Result<Self, Box<dyn Error>> {
		let info: Json = ureq::get(url.join("info")?.as_str()).call()?.into_json()?;
		log::trace!("{}", info);

		let name_json = info
			.get("response")
			.unwrap_or_else(|| panic!("No 'response' in {}/info", url))
			.get("name")
			.unwrap_or_else(|| panic!("No 'name' in {}/info inside json object 'response'", url));

		let name = if name_json.is_string() {
			name_json.as_str().unwrap().to_string()
		}
		else {
			name_json.to_string()
		};

		Ok(Self { name, url, spec })
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
		let valid = response_json["resultSets"].as_array().unwrap().iter().all(|resp| {
			if resp["exists"] == "false" {
				return true;
			}

			resp["results"]
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
								"   ERROR: {} on property path {} ({})",
								e.to_string(),
								e.instance_path.to_string(),
								e
							);
						}
						false
					},
				})
		});

		Some(valid)
	}

	pub fn validate(self) -> Result<BeaconOutput, Box<dyn Error>> {
		let mut entities = Vec::new();

		for entity in &self.spec.entities {
			// Get params
			eprintln!();
			log::info!("Validating {:?}", entity.name);
			let mut replaced_url = self.url.clone();
			replaced_url.set_path(entity.url.path());
			log::debug!("GET {}", replaced_url);

			let valid = self.valid_endpoint(entity, &replaced_url);

			entities.push(EntityOutput {
				name: entity.name.clone(),
				url: replaced_url.clone(),
				valid,
			})
		}

		Ok(BeaconOutput {
			name: self.name,
			url: self.url,
			entities,
		})
	}
}
