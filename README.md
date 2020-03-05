Hole-Punch
==========

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
for segment in segement {
    if let SegementType::Data = segment.segment_type {
        let start = segment.start;
        let end = segment.end;

        let length = end - start;
        do_something_with_data(&mut file, start, length);
    }
}
```

License
-------

Hole-Punch is distrubited under your choice of the MIT license, or Apache
Version 2.0.

TO-DOs
------

The following features are on my "to implement" list, in order of importance:
1. Windows support
2. Fallback mode (reading the entire file manually looking for chunks of 0s)