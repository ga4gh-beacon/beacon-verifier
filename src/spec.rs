use std::collections::{BTreeMap};
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};

use git2::Repository;
use url::Url;

use crate::error::VerifierError;
use crate::framework::Framework;
use crate::interface::{Endpoint, EntryType, RelatedEndpoint};
use crate::{utils, Json};

#[derive(Debug, Clone)]
pub struct Entity {
	pub name: String,
	pub url: Url,
	pub url_single: Option<Url>,
	pub schema: Json,
	pub filtering_terms_url: Option<Url>,
	pub related_endpoints: Option<BTreeMap<String, RelatedEndpoint>>,
}

#[derive(Debug, Clone)]
pub struct Spec {
	pub entities: Vec<Entity>,
	pub entities_names: BTreeMap<String, String>,
	pub configuration_json: Json,
	pub beacon_map_json: Json,
	pub endpoints_json: Json,
	files: BTreeMap<PathBuf, Json>,
}

impl Spec {
	pub fn load(location: &Url) -> Result<Self, VerifierError> {
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

		let mut spec = Self {
			entities: Vec::new(),
			entities_names: BTreeMap::new(),
			configuration_json: Json::Null,
			beacon_map_json: Json::Null,
			endpoints_json: Json::Null,
			files: BTreeMap::new(),
		};

		// Load files
		for entry in walkdir::WalkDir::new(&dir).into_iter().flatten() {
			if entry.path().extension() == Some(OsStr::new("json")) {
				spec.add(entry.path())?;
			}
		}

		// Load configuration
		spec.load_configuration(dir.path());

		// Load entitites
		spec.load_entities(dir.path());

		Ok(spec)
	}

	fn add(&mut self, path: &Path) -> Result<(), VerifierError> {
		log::debug!("Adding JSON file: {:?}", path);
		let file = File::open(path).unwrap();
		let json = serde_json::from_reader(file).map_err(|_| VerifierError::BadJson)?;
		self.files.insert(path.to_path_buf(), json);
		Ok(())
	}

	fn load_configuration(&mut self, base_path: &Path) {
		self.beacon_map_json = self
			.files
			.get(&base_path.join("beaconMap.json"))
			.expect("beaconMap.json not found")
			.clone();
		self.configuration_json = self
			.files
			.get(&base_path.join("beaconConfiguration.json"))
			.expect("beaconConfiguration.json not found")
			.clone();
		self.endpoints_json = self
			.files
			.get(&base_path.join("endpoints.json"))
			.expect("endpoints.json not found")
			.clone();
	}

	fn load_entities(&mut self, base_path: &Path) {
		let mut entities_names = BTreeMap::new();

		let entities_schemas = self.configuration_json["entryTypes"]
			.as_object()
			.unwrap()
			.into_iter()
			.map(|(_, val)| {
				let entry_type: EntryType = serde_json::from_value(val.clone()).unwrap();
				entities_names.insert(entry_type.id.clone(), entry_type.name);
				let mut schema_rel_path = entry_type.default_schema.reference_to_schema_definition;
				if schema_rel_path.starts_with("http") {
					let schema_rel_path_url = Url::parse(&schema_rel_path).unwrap();
					schema_rel_path = Path::new(schema_rel_path_url.path())
						.components()
						.skip(1)
						.collect::<PathBuf>()
						.to_string_lossy()
						.to_string();
				}
				log::debug!("Loading schema on {:?} + {:?}", base_path, schema_rel_path);
				let schema_abs_path = base_path.join(schema_rel_path);
				log::debug!("Loading schema on {:?}", schema_abs_path);
				let schema_file = File::open(schema_abs_path.canonicalize().unwrap()).expect("File not found");
				let schema_json = serde_json::from_reader(schema_file).expect("Bad json schema");
				(entry_type.id, schema_json)
			})
			.collect::<BTreeMap<String, Json>>();

		self.entities_names = entities_names;

		for (_, entity) in self.beacon_map_json["endpointSets"].as_object().unwrap() {
			let endpoint: Endpoint = serde_json::from_value(entity.clone()).unwrap();
			let entity_schema = entities_schemas
				.get(&endpoint.entry_type)
				.unwrap_or_else(|| {
					log::error!(
						"No schema for entry type {}, available schemas = {:?}",
						&endpoint.entry_type,
						entities_schemas.keys()
					);
					panic!();
				})
				.clone();
			self.entities.push(Entity {
				schema: entity_schema,
				name: self
					.entities_names
					.get(&endpoint.entry_type)
					.unwrap_or(&String::from("Unknown entity name"))
					.clone(),
				url: endpoint.root_url,
				url_single: endpoint.single_entry_url,
				filtering_terms_url: endpoint.filtering_terms_url,
				related_endpoints: endpoint.endpoints,
			});
		}
	}

	pub fn validate(&self, _framework: &Framework) -> usize {
		// TODO: Validate model against the framework
		self.entities.len()
	}
}
