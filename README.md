# win-etl

A plattform agnostic library to parse Windows Event Trace Logs without Windows dependencies.

## Usage
```rust
use std::{fs::File, io::BufReader};

use win_etl::Etl;

fn main() {
    let f = File::open("examples/example.etl").unwrap();
    let reader = BufReader::new(f);

    let mut etl = Etl::from_buf(reader).unwrap();

    println!("TraceLogfileHeader: {:#?}", &etl.header);
    for chunk in etl.load_buffers().unwrap() {
        println!("{:#?}", chunk.header);
    }
}
```
