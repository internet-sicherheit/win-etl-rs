# win-etl

A plattform agnostic library to parse Windows Event Trace Logs (ETL) without Windows dependencies.

## Usage
```rust
use std::{fs::File, io::BufReader};

use win_etl::Etl;
use win_etw_event::EtwEvent;

fn main() {
    let f = File::open("examples/example.etl").unwrap();
    let reader = BufReader::new(f);

    let mut etl = Etl::from_buf(reader).unwrap();

    println!("[TraceLogfileHeader]\n{:#?}", &etl.header);
    for chunk in etl.chunks().unwrap() {
        println!("[Chunk Header]\n{:#?}", chunk.header);
        for event in etl.load_events(&chunk).unwrap().into_iter() {
            if let EtwEvent::ModernEvent(e) = event {
                let mut e = e.into_contained_event().unwrap();
                println!(
                    "[Event]\nProcess: {}, Thread: {}, Task: {:?}",
                    e.header.process_id,
                    e.header.thread_id,
                    e.get_event_task_name()
                );
            }
        }
    }
}
```

## License

[MIT](https://choosealicense.com/licenses/mit/)
