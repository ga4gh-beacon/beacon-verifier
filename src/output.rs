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

impl BeaconOutput {
	pub fn summary(&self) {
		self.entities.iter().for_each(|(entity_name, output)| {
			if output.iter().all(|report| report.valid == Some(true)) {
				log::info!("{} \u{2713}", entity_name);
			}
			else {
				log::error!("{} \u{2717}", entity_name);
				for error in output.iter().filter_map(|report| report.error.clone()) {
					log::error!("\t{}", error.trim());
				}
			}
		});
	}
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

	pub fn push(&mut self, report: EndpointReport) {
		match self.results.get_mut(&report.entity_name) {
			Some(endpoints) => {
				endpoints.push(EndpointOutput {
					name: report.name,
					url: report.url.unwrap(),
					valid: report.valid,
					error: report.error.map(|e| e.to_string()),
				});
			},
			None => {
				self.results.insert(
					report.entity_name,
					vec![EndpointOutput {
						name: report.name,
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
	pub entity_name: String,
	pub valid: Option<bool>,
	pub error: Option<VerifierError>,
	pub output: Option<Json>,
	pub url: Option<Url>,
	pub name: String,
}

impl EndpointReport {
	pub fn new(entity_name: &str, name: &str, url: Url) -> Self {
		Self {
			entity_name: entity_name.to_string(),
			name: name.to_string(),
			url: Some(url),
			..Self::default()
		}
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

	pub fn join(self, report2: Self) -> Self {
		if self.valid == Some(true) && (report2.valid.is_none() || !report2.valid.unwrap()) {
			return report2;
		};
		self
	}
}
