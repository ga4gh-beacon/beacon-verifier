use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VerifierError {
	#[error("Request error {0}")]
	RequestError(#[from] reqwest::Error),

	#[error("IO Error")]
	IoError(#[from] std::io::Error),

	#[error("Bad json: {0}")]
	SpecHasBadJson(PathBuf),

	#[error("Bad /info endpoint: {0}")]
	BadInfo(String),

	#[error("Endpoint {0} did not respond")]
	UnresponsiveEndpoint(url::Url),

	#[error("Bad response format (JSON could not be parsed)")]
	ResponseIsNotJson,

	#[error("Unable to compile the schema (use the --spec option)")]
	BadSchema,

	#[error("Bad framework (use the --framework option)")]
	BadFramework,

	#[error("Response does not match the schema: {0}")]
	BadResponse(String),

	#[error("Unexpected HTTP status code")]
	BadStatus,

	#[error("No ids were extracted from the main entity endpoint")]
	NoIds,

	#[error("{0} argument is not a URL")]
	ArgNotURL(&'static str),

	#[error("Granularity is not one of [\"boolean\", \"count\", \"aggregated\", \"record\"]")]
	BadGranularity,
}
