use std::rc::Rc;

use jsonschema::JSONSchema;
use url::Url;

use crate::error::VerifierError;
use crate::interface::{BeaconMetaGranularityResponse, Granularity};
use crate::output::EndpointReport;
use crate::{utils, Json};

pub struct BeaconEndpoint {
	pub entity_name: String,
	pub entity_schema: Rc<JSONSchema>,
	pub name: String,
	pub url: Url,
}

impl BeaconEndpoint {
	pub fn validate(
		self,
		root_url: &Url,
		boolean_json: &Rc<JSONSchema>,
		count_json: &Rc<JSONSchema>,
		result_sets_json: &Rc<JSONSchema>,
	) -> EndpointReport {
		let endpoint_url = utils::url_join(root_url, &self.url);
		log::debug!("GET {}", endpoint_url);

		// Get response
		let response_json = match utils::ping_url(&endpoint_url) {
			Ok(j) => j,
			Err(e) => {
				return EndpointReport::new(&self.entity_name, &self.name, endpoint_url).null(e);
			},
		};

		// Test granularity
		let beacon_meta_response: Result<BeaconMetaGranularityResponse, _> =
			serde_json::from_value(response_json.clone());

		// Test response
		match beacon_meta_response {
			Ok(br) => {
				let valid_against_framework = match br.meta.returned_granularity {
					Granularity::Boolean => self.validate_against_framework(&response_json, boolean_json),
					Granularity::Count => self.validate_against_framework(&response_json, count_json),
					Granularity::Aggregated | Granularity::Record => {
						self.validate_against_framework(&response_json, result_sets_json)
					},
				};
				if let Err(e) = valid_against_framework {
					return EndpointReport::new(&self.entity_name, &self.name, endpoint_url).error(e);
				}

				if Granularity::Record == br.meta.returned_granularity {
					// Compile entity schema
					self.validate_resultset_response(&response_json).url(endpoint_url)
				}
				else {
					EndpointReport::new(&self.entity_name, &self.name, endpoint_url).ok(Some(response_json))
				}
			},
			Err(e) => EndpointReport::new(&self.entity_name, &self.name, endpoint_url).error(e.into()),
		}
	}

	pub fn validate_against_framework(
		&self,
		response_json: &Json,
		response_schema: &Rc<JSONSchema>,
	) -> Result<(), VerifierError> {
		if let Err(e) = utils::valid_schema(response_schema, response_json) {
			return Err(e);
		};
		Ok(())
	}

	pub fn validate_resultset_response(self, response_json: &Json) -> EndpointReport {
		// Case: == 0 results
		if !response_json
			.as_object()
			.expect("JSON is not an object")
			.get("responseSummary")
			.expect("No 'responseSummary' property was found")
			.as_object()
			.expect("'responseSummary' is not an object")
			.get("exists")
			.expect("No 'exists' property found")
			.as_bool()
			.expect("'exists' property is not a bool")
		{
			return EndpointReport::new(&self.entity_name, &self.name, self.url).ok(None);
		}

		// Case: >= 1 results
		log::info!("Verifying results...");
		response_json
			.as_object()
			.expect("JSON is not an object")
			.get("response")
			.expect("No 'response' property was found")
			.as_object()
			.expect("'response' is not an object")
			.get("resultSets")
			.expect("No 'resultSets' property was found")
			.as_array()
			.expect("'resultSets' property is not an array")
			.iter()
			.map(|rs| {
				rs.as_object()
					.expect("resultSet inside 'resultSets' property is not an object")
					.get("results")
					.expect("No 'results' property was found")
					.as_array()
					.expect("'results' property is not an array")
					.iter()
					.map(
						|instance| match utils::valid_schema(&self.entity_schema.clone(), &instance.clone()) {
							Ok(output) => {
								EndpointReport::new(&self.entity_name, &self.name, self.url.clone()).ok(Some(output))
							},
							Err(e) => EndpointReport::new(&self.entity_name, &self.name, self.url.clone()).error(e),
						},
					)
					.fold(
						EndpointReport::new(&self.entity_name, &self.name, self.url.clone()).ok(None),
						EndpointReport::join,
					)
			})
			.fold(
				EndpointReport::new(&self.entity_name, &self.name, self.url.clone()).ok(None),
				EndpointReport::join,
			)
	}
}
