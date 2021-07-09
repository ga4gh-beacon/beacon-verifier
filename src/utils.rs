use std::path::{Path, PathBuf};

pub fn copy_dir_recursively<U: AsRef<Path>, V: AsRef<Path>>(from: U, to: V) -> Result<(), std::io::Error> {
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
