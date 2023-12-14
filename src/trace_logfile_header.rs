use bitflags::bitflags;
use byteorder::{ByteOrder, LittleEndian};
use log::{debug, trace};
use std::io::{Error, Read, Seek};
use widestring::ucstring::U16CString;

/// Header Event describing a Event Trace Log (ETL)
///
/// A TraceLogfileHeader is a synthetic [SYSTEM_TRACE_EVENT](win_etw_event::system_trace_event::SystemTraceEvent).
///
/// <https://www.geoffchappell.com/studies/windows/km/ntoskrnl/inc/shared/evntrace/trace_logfile_header.htm>
#[derive(Clone, Debug)]
#[repr(C)]
pub struct TraceLogfileHeader {
    pub buffer_size: u32,
    version: Version,
    pub provider_version: u32,
    pub number_of_processors: u32,
    /// _WIP Placeholder_
    pub end_time: u64,
    pub time_resolution: u32,
    pub max_file_size: u32,
    pub log_file_mode: LogFileMode,
    pub buffer_written: u32,
    logger_name: String,
    log_file_name: String,
}

/// Version stored in the [TraceLogfileHeader]
///
/// Can be either in the form of [VersionTuple] or u32, depending on the system the ETL was created on.
#[derive(Clone, Copy)]
#[repr(C)]
union Version {
    version_tuple: VersionTuple,
    version: u32,
}
impl core::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let version = unsafe { self.version };
        write!(f, "Version: {version:08X}")
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VersionTuple {
    major_version: u8,
    minor_version: u8,
    sub_version: u8,
    sub_minor_version: u8,
}

// TODO complete Variants
// https://learn.microsoft.com/en-us/windows/win32/etw/logging-mode-constants
bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct LogFileMode: u32 {
        const RealTimeMode = 0x0000_0100;
        const StopOnHybridShutdown = 0x0040_0000;
        const PersistOnHybridShutdown = 0x0080_0000;
        const CompressedMode = 0x4000_0000;
        const NoPerProcessorBuffering = 0x1000_0000;
        const IndependentSessionMode = 0x0800_0000;
    }
}

impl TraceLogfileHeader {
    pub fn parse<T: Read + Seek>(buf: &mut T) -> Result<TraceLogfileHeader, Error> {
        let mut bytes = [0u8; 280];
        buf.read_exact(&mut bytes)?;
        trace!("280 bytes of TraceLogfileHeader read");
        TraceLogfileHeader::parse_slice(&bytes)
    }
    pub fn parse_slice(bytes: &[u8]) -> Result<TraceLogfileHeader, Error> {
        if bytes.len() < 280 {
            return Err(Error::new(
                std::io::ErrorKind::InvalidInput,
                "TraceLogfileHeader has to be at least 280 bytes",
            ));
        }
        trace!("TRACE_LOGFILE_HEADER bytes: {:02X?}", &bytes);

        let buffer_size = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let version = Version {
            version: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        };
        let provider_version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let number_of_processors = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        let end_time = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let time_resolution = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
        let max_file_size = u32::from_le_bytes(bytes[28..32].try_into().unwrap());
        let log_file_mode =
            LogFileMode::from_bits(u32::from_le_bytes(bytes[32..36].try_into().unwrap()))
                .ok_or_else(|| {
                    debug!(
                        "Unexpected LogFileMode: 0x{:08X}",
                        u32::from_le_bytes(bytes[32..36].try_into().unwrap())
                    );
                    Error::new(
                        std::io::ErrorKind::InvalidData,
                        "oh no, unknown LogFileMode encounterd!",
                    )
                })?;
        // .ok_or(Error::new(
        //     std::io::ErrorKind::InvalidData,
        //     "oh no, unknown LogFileMode encounterd!",
        // ))?;
        let buffer_written = u32::from_le_bytes(bytes[36..40].try_into().unwrap());

        let mut u16_vec: Vec<u16> = vec![0; bytes[280..].len() / 2];
        if bytes[280..].len() % 2 != 0 {
            debug!("Uneven length of buffer for u16Cstrings");
        } else {
            LittleEndian::read_u16_into(&bytes[280..], u16_vec.as_mut_slice());
        }

        let cstring_logger_name = U16CString::from_vec_truncate(u16_vec.as_slice());
        let logger_name = cstring_logger_name
            .to_string()
            .map_err(|_| Error::new(std::io::ErrorKind::Other, "Can't convert to Rust String"))?;

        let cstring_log_file_name =
            U16CString::from_vec_truncate(u16_vec.split_off(cstring_logger_name.len() + 1));
        let log_file_name = cstring_log_file_name
            .to_string()
            .map_err(|_| Error::new(std::io::ErrorKind::Other, "Can't convert to Rust String"))?;

        Ok(TraceLogfileHeader {
            buffer_size,
            version,
            provider_version,
            number_of_processors,
            end_time,
            time_resolution,
            max_file_size,
            log_file_mode,
            buffer_written,
            logger_name,
            log_file_name,
        })
    }
}
