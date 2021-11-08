use jsonschema::JSONSchema;

use crate::error::VerifierError;
use crate::output::EndpointReport;
use crate::{utils, Json};

pub struct BeaconEndpoint {}

impl BeaconEndpoint {
	pub fn validate_against_framework(response_json: &Json, framework_json: &Json) -> Result<(), VerifierError> {
		let result_sets_schema = match jsonschema::JSONSchema::options()
			.with_meta_schemas()
			.compile(framework_json)
		{
			Ok(schema) => schema,
			Err(e) => {
				log::error!("{:?}", e);
				return Err(VerifierError::BadSchema);
			},
		};
		if let Err(e) = utils::valid_schema(&result_sets_schema, response_json) {
			return Err(e);
		};
		Ok(())
	}

	pub fn validate_resultset_response(response_json: &Json, schema: &JSONSchema) -> EndpointReport {
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
			return EndpointReport::new().ok(None);
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
					.map(|instance| match utils::valid_schema(schema, &instance.clone()) {
						Ok(output) => EndpointReport::new().ok(Some(output)),
						Err(e) => EndpointReport::new().error(e),
					})
					.fold(EndpointReport::new().ok(None), EndpointReport::join)
			})
			.fold(EndpointReport::new().ok(None), EndpointReport::join)
	}
}
