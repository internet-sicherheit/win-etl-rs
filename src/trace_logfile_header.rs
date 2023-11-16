#[derive(Clone)]
#[repr(C)]
pub struct TraceLogfileHeader {
    buffer_size: u32,
    version: Version,
    provider_version: u32,
    number_of_processors: u32,
    /// _WIP Placeholder_
    end_time: u64,
    time_resolution: u32,
    max_file_size: u32,
    log_file_mode: LogFileMode,
    buffer_written: u32,
}

#[derive(Clone, Copy)]
#[repr(C)]
union Version {
    version_tuple: VersionTuple,
    version: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VersionTuple {
    major_version: u8,
    minor_version: u8,
    sub_version: u8,
    sub_minor_version: u8,
}

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum LogFileMode {
    Dummy = 0x00,
}
