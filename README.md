# win-etl

A plattform agnostic library to parse Windows Event Trace Logs without Windows dependencies.

## Usage
```rust
use std::{fs::File, io::BufReader};

use win_etl::Etl;

fn main() {
    let f = File::open("examples/example.etl").unwrap();
    let reader = BufReader::new(f);

    let event_log = Etl::from_buf(reader).unwrap();

    for chunk in event_log.chunks {
        println!("{:#?}", chunk.header);
    }
}
```
