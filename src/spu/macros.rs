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
