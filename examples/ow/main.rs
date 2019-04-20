
use std::env;
use std::path::PathBuf;

fn parse_arg() -> Option<PathBuf> {
	let mut args_os = env::args_os();
	args_os.next();
	args_os.next().map(|path| path.into())
}

fn main() {
	match parse_arg() {
		None => {
			eprintln!("Give the path to dumped Overwatch binary.");
			return;
		},
		Some(path) => {
			let filemap = pelite::FileMap::open(&path).unwrap();
			parse(filemap.as_ref());
		},
	}
}

fn parse(image: &[u8]) {
	use pelite::pe64::*;
	let bin = PeFile::from_bytes(image).unwrap();
	misc::print(bin);
	globals::print(bin);
}

mod globals;
mod misc;
