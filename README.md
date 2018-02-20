# extar

extar is a simple library for reading tar archives. Its intended usage is for
out-of-core or external processing, where it is advisable to seek as much as
possible to avoid reading and paging.

`BufferedTarFile` currently exposes one iterator, `RawTarEntries`. As its name
suggests, it yields the bare minimum information that the application may find
useful: the header offset, the filename, the file offset, and the file size.
The application is responsible for actually reading the file.

```rust,no_run
extern crate extar;

use extar::*;
use std::fs::{File};
use std::path::{PathBuf};

fn main() {
  let path = PathBuf::new("ILSVRC2012_img_train.tar");
  let file = File::open(&path).unwrap();
  let mut tar = BufferedTarFile::new(file);
  let file_count = tar.raw_entries().count();
  assert_eq!(file_count, 1281167);
}
```
