extern crate extar;

use extar::*;

use std::env;
use std::fs::{File};
use std::path::{PathBuf};

fn main() {
  let args: Vec<_> = env::args().collect();
  if args.len() < 2 {
    println!("usage: {} [tarfile]", args[0]);
    return;
  }

  let path = PathBuf::from(&args[1]);
  let file = File::open(&path).unwrap();
  let mut tar = BufferedTarFile::new(file);
  let mut entry_count = 0;
  for _entry in tar.raw_entries() {
    entry_count += 1;
  }
  println!("file count: {}", entry_count);
}
