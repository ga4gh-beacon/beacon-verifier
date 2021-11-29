use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use git2::Repository;
use jsonschema::JSONSchema;
use url::Url;

use crate::endpoint::BeaconEndpoint;
use crate::error::VerifierError;
use crate::interface::{Endpoint, EntryType, RelatedEndpoint};
use crate::utils::replace_vars;
use crate::{utils, Json};

#[derive(Debug, Clone)]
pub struct Entity {
	pub name: String,
	pub url: Url,
	pub url_single: Option<Url>,
	pub schema: Rc<JSONSchema>,
	pub filtering_terms_url: Option<Url>,
	pub related_endpoints: Option<BTreeMap<String, RelatedEndpoint>>,
}

#[derive(Debug, Clone)]
pub struct Model {
	pub entities: Vec<Entity>,
	pub entities_names: BTreeMap<String, String>,
	pub configuration_json: Json,
	pub beacon_map_json: Json,
	pub endpoints_json: Json,
	files: BTreeMap<PathBuf, Json>,
}

impl Model {
	pub fn load(location: &Url) -> Result<Self, VerifierError> {
		let dir = tempfile::tempdir().expect("Could not create temporary directory");

		if location.scheme() == "file" {
			log::debug!("COPYING {} to {:?}", location.path(), dir.path());
			utils::copy_dir_recursively(location.path(), &dir).expect("Copy dir recursively failed");
		}
		else {
			// Parse model repo URL
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

		let mut model = Self {
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
				model.add(entry.path())?;
			}
		}

		// Load configuration
		model.load_configuration(dir.path());

		// Load entitites
		model.load_entities(dir.path());

		Ok(model)
	}

	fn add(&mut self, path: &Path) -> Result<(), VerifierError> {
		log::debug!("Adding JSON file: {:?}", path);
		let file = File::open(path).unwrap();
		let json = serde_json::from_reader(file).map_err(|_| VerifierError::ModelHasBadJson(path.to_path_buf()))?;
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
				schema: utils::compile_schema(&entity_schema),
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

	fn build_endpoint(
		entity_name: String,
		entity_schema: Rc<JSONSchema>,
		name: String,
		url: &Url,
		vars: Vec<(&str, &str)>,
	) -> BeaconEndpoint {
		let replaced_url = replace_vars(url, vars);
		BeaconEndpoint {
			entity_name,
			entity_schema,
			name,
			url: replaced_url,
		}
	}

	pub fn endpoints(self, root_url: &Url) -> Vec<BeaconEndpoint> {
		self.entities
			.iter()
			.flat_map(|entity| {
				let mut endpoints = Vec::new();
				let entity_schema = &entity.schema;

				endpoints.push(Self::build_endpoint(
					entity.name.clone(),
					entity_schema.clone(),
					format!("{} all entries", entity.name.clone()),
					&entity.url,
					vec![],
				));

				let ids = utils::get_ids(root_url, &entity.url);

				if let Ok(ids) = ids {
					if let Some(url_single) = &entity.url_single {
						endpoints.extend(ids.iter().take(1).map(|id| {
							Self::build_endpoint(
								entity.name.clone(),
								entity_schema.clone(),
								format!("{} single entry", entity.name.clone()),
								url_single,
								vec![("id", id)],
							)
						}));
					}

					// TODO: Filtering terms
					// if let Some(filtering_terms_url) = &entity.filtering_terms_url {
					// 	let available_filtering_terms = utils::get_filtering_terms(filtering_terms_url);
					// 	endpoints.extend(available_filtering_terms.iter().take(1).map(|filtering_term| {
					// 		Model::build_endpoint(
					// 			entity.name,
					// 			entity.schema,
					// 			format!("{} filtering terms", entity.name.clone()),
					// 			filtering_terms_url,
					// 			vec![("id", &id)]
					// 		)
					// 	}));
					// }

					if let Some(related_endpoints) = &entity.related_endpoints {
						endpoints.extend(related_endpoints.iter().flat_map(|(_, related_endpoint)| {
							ids.iter().take(1).map(|id| {
								let default_entity_name = "Unknown entity".to_string();
								let related_entity_name = self
									.entities_names
									.get(&related_endpoint.returned_entry_type)
									.unwrap_or(&default_entity_name);
								let name = format!("{} related with a {}", related_entity_name, entity.name.clone());
								let related_entity_schema = &self
									.entities
									.iter()
									.find(|e| &e.name == related_entity_name)
									.unwrap()
									.schema;
								Self::build_endpoint(
									entity.name.clone(),
									related_entity_schema.clone(),
									name,
									&related_endpoint.url,
									vec![("id", id)],
								)
							})
						}));
					}
				}

				endpoints
			})
			.collect()
	}
}
