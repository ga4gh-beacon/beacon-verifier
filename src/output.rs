use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::VerifierError;
use crate::Json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointOutput {
	pub name: String,
	pub url: Url,
	pub valid: Option<bool>,
	pub error: Option<String>,
}

pub struct Output {
	results: Vec<EndpointOutput>,
}

impl Output {
	pub const fn new() -> Self {
		Self { results: Vec::new() }
	}

	pub fn push(&mut self, report: EndpointReport) {
		self.results.push(EndpointOutput {
			name: report.name.unwrap(),
			url: report.url.unwrap(),
			valid: report.valid,
			error: report.error.map(|e| e.to_string()),
		});
	}

	pub fn finish(self) -> Vec<EndpointOutput> {
		log::debug!("Compiling results...");
		self.results
	}
}

#[derive(Default)]
pub struct EndpointReport {
	pub valid: Option<bool>,
	pub error: Option<VerifierError>,
	pub output: Option<Json>,
	pub url: Option<Url>,
	pub name: Option<String>,
}

impl EndpointReport {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn null(mut self, e: VerifierError) -> Self {
		self.valid = None;
		self.error = Some(e);
		self
	}

	pub fn error(mut self, e: VerifierError) -> Self {
		self.valid = Some(false);
		self.error = Some(e);
		self
	}

	pub fn ok(mut self, j: Option<Json>) -> Self {
		self.valid = Some(true);
		self.output = j;
		self
	}

	pub fn url(mut self, url: Url) -> Self {
		self.url = Some(url);
		self
	}

	pub fn name(mut self, name: &str) -> Self {
		self.name = Some(name.into());
		self
	}

	pub fn join(self, report2: Self) -> Self {
		if let Some(true) = self.valid {
			if report2.valid.is_none() || !report2.valid.unwrap() {
				return report2;
			}
		};
		self
	}
}
