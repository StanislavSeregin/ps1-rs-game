#[macro_export]
macro_rules! include_bytes_skip_vag_header {
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

#[macro_export]
macro_rules! include_bytes_skip_take {
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
