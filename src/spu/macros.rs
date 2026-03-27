/// Embed a VAG file, stripping the 48-byte header at compile time.
///
/// Returns `&'static [u8]` containing only the ADPCM payload.
#[macro_export]
macro_rules! include_vag {
    ($path:expr) => {{
        const RAW: &[u8] = include_bytes!($path);
        const LEN: usize = RAW.len() - 48;
        const DATA: [u8; LEN] = {
            let mut arr = [0u8; LEN];
            let mut i = 0;
            while i < LEN {
                arr[i] = RAW[i + 48];
                i += 1;
            }
            arr
        };
        &DATA
    }};
}

/// Embed a binary file, skipping `$skip` bytes from the start.
///
/// Optionally takes a third argument to limit the number of bytes kept.
#[macro_export]
macro_rules! include_bytes_skip {
    ($path:expr, $skip:expr) => {{
        const RAW: &[u8] = include_bytes!($path);
        const SKIP: usize = $skip;
        const LEN: usize = RAW.len() - SKIP;
        const DATA: [u8; LEN] = {
            let mut arr = [0u8; LEN];
            let mut i = 0;
            while i < LEN {
                arr[i] = RAW[i + SKIP];
                i += 1;
            }
            arr
        };
        &DATA
    }};
    ($path:expr, $skip:expr, $take:expr) => {{
        const RAW: &[u8] = include_bytes!($path);
        const SKIP: usize = $skip;
        const LEN: usize = $take;
        const DATA: [u8; LEN] = {
            let mut arr = [0u8; LEN];
            let mut i = 0;
            while i < LEN {
                arr[i] = RAW[i + SKIP];
                i += 1;
            }
            arr
        };
        &DATA
    }};
}
