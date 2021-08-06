use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconOutput {
	pub name: String,
	pub url: Url,
	pub entities: Vec<EndpointOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointOutput {
	pub name: String,
	pub url: Url,
	pub valid: Option<bool>,
	pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
	pub entry_type: String,
	pub open_apiendpoints_definition: Option<PathBuf>,
	pub root_url: Url,
	pub single_entry_url: Option<Url>,
	pub filtering_terms_url: Option<Url>,
	pub endpoints: Option<HashMap<String, RelatedEndpoint>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedEndpoint {
	pub returned_entry_type: String,
	pub url: Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryType {
	pub id: String,
	pub name: String,
	pub ontology_term_for_this_type: OntologyTerm,
	pub part_of_specification: String,
	pub default_schema: DefaultSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OntologyTerm {
	pub id: String,
	pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultSchema {
	pub reference_to_schema_definition: String,
}
