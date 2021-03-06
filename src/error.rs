use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VerifierError {
	#[error("Request error {0}")]
	RequestError(#[from] reqwest::Error),

	#[error("IO Error")]
	IoError(#[from] std::io::Error),

	#[error("Bad json: {0}")]
	ModelHasBadJson(PathBuf),

	#[error("Bad /info endpoint: {0}")]
	BadInfo(String),

	#[error("Endpoint {0} did not respond")]
	UnresponsiveEndpoint(url::Url),

	#[error("Bad response format (JSON could not be parsed)")]
	ResponseIsNotJson,

	#[error("Unable to compile the schema (use the --model option)")]
	BadSchema,

	#[error("Bad framework (use the --framework option)")]
	BadFramework,

	#[error("Response does not match the schema: {0}")]
	BadResponse(String),

	#[error("Unexpected HTTP status code")]
	BadStatus,

	// TODO: Use errors
	// #[error("No ids were extracted from the main entity endpoint")]
	// NoIds,
	#[error("Error deserializing JSON: {0}")]
	SerdeJsonError(#[from] serde_json::Error),
}
