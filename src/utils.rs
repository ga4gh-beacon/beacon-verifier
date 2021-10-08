use std::path::{Path, PathBuf};

use url::Url;

use crate::error::VerifierError;
use crate::interface::FilteringTerm;
use crate::output::EndpointReport;
use crate::{error, Json};

pub fn copy_dir_recursively<U: AsRef<Path>, V: AsRef<Path>>(from: U, to: V) -> Result<(), VerifierError> {
	let mut stack = vec![PathBuf::from(from.as_ref())];

	let output_root = PathBuf::from(to.as_ref());
	let input_root = PathBuf::from(from.as_ref()).components().count();

	while let Some(working_path) = stack.pop() {
		log::debug!("process: {:?}", &working_path);

		// Generate a relative path
		let src: PathBuf = working_path.components().skip(input_root).collect();

		// Create a destination if missing
		let dest = if src.components().count() == 0 {
			output_root.clone()
		}
		else {
			output_root.join(&src)
		};
		if std::fs::metadata(&dest).is_err() {
			log::debug!(" mkdir: {:?}", dest);
			std::fs::create_dir_all(&dest)?;
		}

		for entry in std::fs::read_dir(working_path)? {
			let entry = entry?;
			let path = entry.path();
			if path.is_dir() {
				stack.push(path);
			}
			else {
				match path.file_name() {
					Some(filename) => {
						let dest_path = dest.join(filename);
						log::debug!("  copy: {:?} -> {:?}", &path, &dest_path);
						std::fs::copy(&path, &dest_path)?;
					},
					None => {
						log::error!("failed: {:?}", path);
					},
				}
			}
		}
	}

	Ok(())
}

pub fn ping_url(endpoint_url: &Url) -> Result<Json, VerifierError> {
	// Query endpoint
	let client = reqwest::blocking::Client::new();

	let response = match client.get(endpoint_url.clone()).send() {
		Ok(response) if response.status().is_success() => response,
		Ok(response) => {
			if response.status().as_u16() == 405 {
				match client.post(endpoint_url.clone()).send() {
					Ok(response) if response.status().is_success() => response,
					Ok(_) => return Err(VerifierError::UnresponsiveEndpoint(endpoint_url.clone())),
					Err(e) => return Err(VerifierError::RequestError(e)),
				}
			}
			else {
				return Err(VerifierError::UnresponsiveEndpoint(endpoint_url.clone()));
			}
		},
		Err(e) => {
			return if e.is_status() {
				log::error!("{:?}", e);
				Err(error::VerifierError::BadStatus)
			}
			else {
				log::error!("{:?}", e);
				Err(error::VerifierError::RequestError(e))
			};
		},
	};

	let response_json = match response.json() {
		Ok(response_json) => response_json,
		Err(e) => {
			log::error!("{:?}", e);
			return Err(VerifierError::ResponseIsNotJson);
		},
	};

	Ok(response_json)
}

pub fn url_join(url1: &Url, url2: &Url) -> Url {
	let mut replaced_url = url1.clone();
	let new_path: PathBuf = PathBuf::from(replaced_url.path())
		.components()
		.chain(Path::new(url2.path()).components().skip(1))
		.collect();
	replaced_url.set_path(new_path.to_str().unwrap_or(""));
	replaced_url
}

pub fn replace_vars(url: &Url, vars: Vec<(&str, &str)>) -> Url {
	let mut url_string = url.to_string();
	for (var_key, var_val) in vars {
		url_string = url.to_string().replace(&format!("%7B{}%7D", var_key), var_val);
	}
	Url::parse(&url_string).unwrap()
}

pub fn get_filtering_terms(url: &Url) -> Vec<FilteringTerm> {
	// Query endpoint
	match reqwest::blocking::get(url.as_str()) {
		Ok(response) => {
			let j = response.json().unwrap();
			serde_json::from_value(j).unwrap()
		},
		Err(_) => Vec::new(),
	}
}

pub fn get_ids(report: &EndpointReport) -> Option<String> {
	if report.valid.is_none() || !report.valid.unwrap() || report.output.is_none() {
		return None;
	}
	let output = report.output.clone().unwrap();
	log::debug!("get_ids from: {}", output);
	output["id"].as_str().map(std::string::ToString::to_string)
}

#[cfg(test)]
mod tests {

	use url::Url;

	use crate::utils::replace_vars;

	#[test]
	fn test_replace_vars() {
		let replaced = replace_vars(
			&Url::parse("https://google.com/biosamples/{id}").unwrap(),
			vec![("id", "my_id")],
		);
		assert_eq!(replaced.to_string(), "https://google.com/biosamples/my_id");
	}
}
