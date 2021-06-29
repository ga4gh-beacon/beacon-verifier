use std::{
	collections::BTreeMap,
	error::Error,
	fs::File,
	path::{Path, PathBuf},
};

use crate::utils;

type Json = serde_json::Value;

pub struct Model {
	files: BTreeMap<PathBuf, Json>,
}

impl Model {
	pub fn new() -> Self {
		Self { files: BTreeMap::new() }
	}

	pub fn _new_with_config() {
		unimplemented!()
	}

	pub fn add(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
		let file = File::open(path)?;
		let json = serde_json::from_reader(file)?;
		self.files.insert(path.to_path_buf(), json);
		Ok(())
	}

	pub fn validate(&mut self) {
		self.set_absolute_paths();
		self.validate_schemas();
	}

	pub fn set_absolute_paths(&mut self) {
		let mut files_absolutes = BTreeMap::new();
		for (path, instance) in &self.files {
			let mut instance = instance.clone();
			if self.should_process(path.as_path()) {
				// Schema
				match instance["$schema"].as_str() {
					Some(schema_path_str) => {
						let schema_path = utils::fix_path(schema_path_str, path.to_str().unwrap());
						match schema_path {
							Ok(schema_path) => {
								paris::info!("SCHEMA PATH: {}", schema_path);
								instance["$schema"] = serde_json::Value::String(schema_path);
							}
							Err(_) => {
								paris::warn!(
									"<yellow>FILE NOT FOUND IN:</> {:?} (file = {:?})",
									utils::normalize_path(Path::new(path.to_str().unwrap())),
									utils::normalize_path(Path::new(schema_path_str))
								);
							}
						}
					}
					None => {
						paris::info!("{:?}", path);
					}
				}

			// match File::open(example_path.parent().unwrap().join(&schema_path)) {
			// 	Ok(schema_file) => {
			// 		// Fix references
			// 		// paris::info!(
			// 		// 	"FILES: validator: {:?}",
			// 		// 	example_path.parent().unwrap().join(schema_path)
			// 		// );
			// 		// paris::info!("FILES: example: {:?}", example_path);
			// 		let mut schema: Value = serde_json::from_reader(&schema_file)?;

			// 		deep_keys(
			// 			&mut instance,
			// 			example_path
			// 				.parent()
			// 				.unwrap()
			// 				.join(&schema_path)
			// 				.parent()
			// 				.unwrap()
			// 				.to_str()
			// 				.unwrap(),
			// 		);

			// 		deep_keys(
			// 			&mut schema,
			// 			&format!(
			// 				"file://{}",
			// 				example_path
			// 					.parent()
			// 					.unwrap()
			// 					.join(&schema_path)
			// 					.parent()
			// 					.unwrap()
			// 					.to_str()
			// 					.unwrap()
			// 			),
			// 		);

			// 		let mut scope = json_schema::Scope::new();
			// 		let scoped_schema = scope.compile_and_return(schema.clone(), false).unwrap();
			// 		let val = scoped_schema.validate(&instance);
			// 		if val.is_valid() {
			// 			paris::success!("<green>VALID:</>     {:?}", example_path.canonicalize().unwrap(),);
			// 		} else {
			// 			paris::error!(
			// 				"<red>NOT VALID:</> {:?} ({:?})",
			// 				example_path.canonicalize().unwrap(),
			// 				Path::new(&schema_path).file_stem().unwrap(),
			// 			);
			// 		}
			// 		for e in val.errors {
			// 			paris::info!("    <red>ERROR:</> {:?}", e);
			// 		}
			// 		for e in val.missing {
			// 			paris::info!("    <yellow>MISSING:</> {}", e);
			// 		}
			// 		if let Some(e) = val.replacement {
			// 			paris::info!("    <yellow>REPLACEMENT:</> {}", e);
			// 		}
			// 	}
			// 	Err(_) => {
			// 		paris::warn!("<yellow>NOT FOUND:</> {:?}", normalize_path(&example_path));
			// 	}
			// }
			} else {
				paris::info!("Skipping...");
			}
			files_absolutes.insert(path.clone(), instance);
		}
		self.files = files_absolutes;
	}

	fn validate_schemas(&self) {}

	fn should_process(&self, path: &Path) -> bool {
		let path = path.file_stem().unwrap().to_string_lossy().to_string().to_lowercase();
		!path.starts_with("tr_") && !path.starts_with("xx_")
	}
}
