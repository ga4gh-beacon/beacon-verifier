use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};

use git2::Repository;
use url::Url;

use crate::{utils, Json};

#[derive(Debug, Clone)]
pub struct Framework {
	pub configuration_json: Json,
	pub beacon_map_json: Json,
	pub entry_types_json: Json,
	files: HashMap<PathBuf, Json>,
}

impl Framework {
	pub fn load(location: &Url) -> Result<Self, Box<dyn Error>> {
		let dir = tempfile::tempdir().expect("Could not create temporary directory");

		if location.scheme() == "file" {
			log::debug!("COPYING {} to {:?}", location.path(), dir.path());
			utils::copy_dir_recursively(location.path(), &dir).expect("Copy dir recursively failed");
		}
		else {
			// Parse spec repo URL
			assert_eq!(
				location.domain().unwrap_or(""),
				"github.com",
				"Only repos hosted on github.com are supported"
			);
			let mut url_iter = Path::new(location.path()).components().skip(1);
			let owner = url_iter.next().unwrap().as_os_str().to_string_lossy().to_string();
			let repo = url_iter.next().unwrap().as_os_str().to_string_lossy().to_string();
			let path: PathBuf = url_iter.collect();

			log::debug!("Downloading repo {} from {}", repo, owner);
			log::debug!("Path inside repo = {:?}", path);

			// Clone repo to tempdir
			let repo_url = format!("https://github.com/{owner}/{repo}", owner = owner, repo = repo);
			let full_git_dir = tempfile::tempdir().expect("Could not create temporary directory");
			Repository::clone(&repo_url, full_git_dir.path()).expect("Unable to clone repository");

			// Copy subfolder to the final tempdir
			utils::copy_dir_recursively(full_git_dir.path().join(path), dir.path()).unwrap();
		}

		let mut framework = Self {
			configuration_json: Json::Null,
			beacon_map_json: Json::Null,
			entry_types_json: Json::Null,
			files: HashMap::new(),
		};

		// Load files
		for entry in walkdir::WalkDir::new(&dir).into_iter().flatten() {
			if entry.path().extension() == Some(OsStr::new("json")) {
				framework.add(entry.path())?;
			}
		}

		// Load configuration
		framework.load_configuration(dir.path());

		Ok(framework)
	}

	fn add(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
		log::debug!("Adding JSON file: {:?}", path);
		let file = File::open(path)?;
		let json = serde_json::from_reader(file)?;
		self.files.insert(path.to_path_buf(), json);
		Ok(())
	}

	fn load_configuration(&mut self, base_path: &Path) {
		self.beacon_map_json = self
			.files
			.get(&base_path.join("responses").join("beaconMapResponse.json"))
			.expect("beaconMapResponse.json not found")
			.clone();
		self.configuration_json = self
			.files
			.get(&base_path.join("responses").join("beaconConfigurationResponse.json"))
			.expect("beaconConfigurationResponse.json not found")
			.clone();
		self.entry_types_json = self
			.files
			.get(&base_path.join("responses").join("beaconEntryTypesResponse.json"))
			.expect("beaconEntryTypesResponse.json not found")
			.clone();
	}
}
