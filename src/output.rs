use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::VerifierError;
use crate::Json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconOutput {
	pub name: String,
	pub url: Url,
	pub last_updated: NaiveDateTime,
	pub entities: BTreeMap<String, Vec<EndpointOutput>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointOutput {
	pub name: String,
	pub url: Url,
	pub valid: Option<bool>,
	pub error: Option<String>,
}

pub struct Output {
	results: BTreeMap<String, Vec<EndpointOutput>>,
}

impl Output {
	pub fn new() -> Self {
		Self {
			results: BTreeMap::new(),
		}
	}

	pub fn push(&mut self, entity: String, report: EndpointReport) {
		match self.results.get_mut(&entity) {
			Some(endpoints) => {
				endpoints.push(EndpointOutput {
					name: report.name.unwrap(),
					url: report.url.unwrap(),
					valid: report.valid,
					error: report.error.map(|e| e.to_string()),
				});
			},
			None => {
				self.results.insert(
					entity,
					vec![EndpointOutput {
						name: report.name.unwrap(),
						url: report.url.unwrap(),
						valid: report.valid,
						error: report.error.map(|e| e.to_string()),
					}],
				);
			},
		}
	}

	pub fn finish(self) -> BTreeMap<String, Vec<EndpointOutput>> {
		log::debug!("Compiling results...");
		self.results
			.into_iter()
			.map(|(entity_name, mut output)| {
				output.sort_by_key(|k| k.name.clone());
				(entity_name, output)
			})
			.collect()
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
