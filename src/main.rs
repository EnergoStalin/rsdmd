use std::{env, fs, io::ErrorKind, path::Path, thread};

use inotify::{Events, Inotify, WatchMask};
use regex::Regex;

fn check_part_files<P: AsRef<Path>>(path: &P) -> bool {
	let dir = fs::read_dir(path).unwrap();
	for entry in dir {
		let path = entry.unwrap().path();
		if path.is_file() && path.extension().unwrap_or_default() == "part" {
			return true;
		}
	}

	return false;
}

fn wait_for_no_part_files<P: AsRef<Path>>(path: &P) {
	loop {
		if !check_part_files(path) {
			break;
		}

		thread::sleep(std::time::Duration::from_millis(1000));
	}
}

unsafe fn read_events(watcher: &mut Inotify) -> std::io::Result<Events<'static>> {
	static mut BUFFER: &mut [u8; 1024] = &mut [0; 1024];
	loop {
		match watcher.read_events(BUFFER) {
			Ok(events) => return Ok(events),
			Err(error) if error.kind() == ErrorKind::WouldBlock => continue,
			Err(error) => return Err(error),
		};
	};
}

fn main() {
	let args: Vec<String> = env::args().collect();

	let watch_path = Path::new(args.get(1).expect("No source path specified"));
	let move_to = Path::new(args.get(2).expect("No destination path specified"));
	let pattern_string = args.get(3).expect("No file pattern specified");

	let pattern = Regex::new(pattern_string).expect("Invalid regex expression");

	if !move_to.exists() {
		panic!("Destination path does not exist");
	}

	if !watch_path.exists() {
		panic!("Watch path does not exist")
	}

	let mut watcher = Inotify::init().expect("Error while initializing directory watcher instance");

	watcher
		.watches()
		.add(watch_path, WatchMask::CREATE)
		.expect("Failed adding file to watch list");

	loop {
		let directory_events = unsafe { read_events(&mut watcher) };

		for event in directory_events.unwrap() {
			let path = watch_path.join(match event.name {
				Some(name) => name,
				None => continue,
			});

			if !pattern.is_match(path.to_str().unwrap()) {
				continue;
			}

			println!("Matched: {}", path.display());

			let name = path.file_name().unwrap();
			let new = Path::new(move_to).join(name);

			println!("Waiting {} to download.", path.display());

			wait_for_no_part_files(&watch_path);

			println!("Moving {} to {}", path.display(), move_to.display());

			fs::rename(path, new).expect("Failed to move file");
		}
	}
}
