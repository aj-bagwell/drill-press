Hole-Punch
==========
![Crates.io](https://img.shields.io/crates/v/hole-punch?style=flat-square&logo=rust)
![License](https://img.shields.io/crates/l/hole-punch?style=flat-square)
![Unsafe](https://img.shields.io/badge/unsafe-very%20yes-important?style=flat-square)
![Maintenance](https://img.shields.io/maintenance/yes/2020?style=flat-square)


A (wip) dead simple, cross platform crate for finding the locations of holes in
sparse files.

Currently only supports Unix, but Windows support is coming soon.


Usage
-----

```rust
use std::fs::File;
use hole_punch::*;

let mut file = File::open("a big sparse file");
let segments = file.scan_chunks().expect("Unable to scan chunks");
for segment in segment {
    if let SegmentType::Data = segment.segment_type {
        let start = segment.start;
        let end = segment.end;

        let length = end - start;
        do_something_with_data(&mut file, start, length);
    }
}
```

License
-------

Hole-Punch is distributed under your choice of the MIT license, or Apache
Version 2.0.

TO-DOs
------

The following features are on my "to implement" list, in order of importance:
1. Windows support
2. Fallback mode (reading the entire file manually looking for chunks of 0s)