use std::io::{Read, Seek};

pub mod wmi_buffer;

pub struct Etl {
    chunks: Vec<wmi_buffer::Buffer>,
}

impl Etl {
    pub fn from<T: Read + Seek>(buf: T) -> Etl {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        #[allow(clippy::assertions_on_constants)]
        assert!(true);
    }
}
