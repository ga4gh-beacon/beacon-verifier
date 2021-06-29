use std::{
	error::Error,
	path::{Component, Path, PathBuf},
};

use url::{ParseError, Url};

pub fn fix_path(path_str: &str, in_file: &str) -> Result<String, Box<dyn Error>> {
	let url = Url::parse(path_str);
	match url {
		Ok(url) => Ok(url.to_string()),
		Err(e) => match e {
			ParseError::RelativeUrlWithoutBase => {
				let in_file_path = Path::new(in_file).parent().unwrap();
				let path = in_file_path.join(Path::new(path_str));
				Ok(path.canonicalize()?.to_string_lossy().to_string())
			}
			_ => {
				panic!("{}", e);
			}
		},
	}
}

pub fn normalize_path(path: &Path) -> PathBuf {
	let mut components = path.components().peekable();
	let mut ret = components.peek().copied().map_or_else(PathBuf::new, |c| {
		components.next();
		PathBuf::from(c.as_os_str())
	});

	for component in components {
		match component {
			Component::Prefix(..) => unreachable!(),
			Component::RootDir => {
				ret.push(component.as_os_str());
			}
			Component::CurDir => {}
			Component::ParentDir => {
				ret.pop();
			}
			Component::Normal(c) => {
				ret.push(c);
			}
		}
	}
	ret
}
