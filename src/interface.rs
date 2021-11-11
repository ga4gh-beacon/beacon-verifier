use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
	pub entry_type: String,
	pub open_apiendpoints_definition: Option<PathBuf>,
	pub root_url: Url,
	pub single_entry_url: Option<Url>,
	pub filtering_terms_url: Option<Url>,
	pub endpoints: Option<BTreeMap<String, RelatedEndpoint>>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FilteringTermType {
	OntologyTerm,
	Alphanumeric,
	Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilteringTerm {
	ft_type: FilteringTermType,
	pub url: Url,
	id: String,
	label: Option<String>,
	scope: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
	Boolean,
	Count,
	Aggregated,
	Record,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconResultSetResponse {
	pub response: ResultSetResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultSetResponse {
	pub result_sets: Vec<ResultSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSet {
	pub results: Vec<EntityResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum EntityResult {
	General { id: String },
	Variant { variant_internal_id: String },
	Cohort { cohort_id: String },
}

impl EntityResult {
	pub fn id(&self) -> String {
		match self {
			EntityResult::General { id } => id.clone(),
			EntityResult::Variant { variant_internal_id } => variant_internal_id.clone(),
			EntityResult::Cohort { cohort_id } => cohort_id.clone(),
		}
	}
}

/// Extract granularity

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconMetaGranularityResponse {
	pub meta: MetaGranularityResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaGranularityResponse {
	pub returned_granularity: Granularity,
}
