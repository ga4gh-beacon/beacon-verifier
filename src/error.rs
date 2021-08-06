use thiserror::Error;

#[derive(Error, Debug)]
pub enum VerifierError {
	#[error("ureq Error")]
	UreqError(#[from] Box<ureq::Error>),

	#[error("IO Error")]
	IoError(#[from] std::io::Error),

	#[error("File Error")]
	BadJson,

	#[error("Bad /info endpoint: {0}")]
	BadInfo(String),

	#[error("Endpoint {0} did not respond")]
	UnresponsiveEndpoint(url::Url),

	#[error("Bad response format (JSON could not be parsed)")]
	ResponseIsNotJson,

	#[error("Unable to compile the schema (use the --spec option)")]
	BadSchema,

	#[error("No resultsSets property in the response")]
	NoResultSets,

	#[error("Bad framework (use the --framework option)")]
	BadFramework,

	#[error("Response does not match the schema: {0}")]
	BadResponse(String),
}
