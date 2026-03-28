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

/// Declare a [`SoundProject`](crate::spu::SoundProject) with named samples.
///
/// Generates a unit struct whose associated constants give type-safe access
/// to both the project data (`DATA`) and each sample id.
///
/// ```ignore
/// crate::sound_project! {
///     pub PROJECT {
///         samples: [
///             KICK  => crate::include_vag!("samples/kick.vag"),
///             SNARE => crate::include_vag!("samples/snare.vag"),
///         ],
///         layout: VoiceLayout::new((0, 16), (16, 8)),
///     }
/// }
///
/// // Usage:
/// // e.load_project(&PROJECT::DATA);
/// // Cell::note(PROJECT::KICK, Pitch(0x1000));
/// ```
#[macro_export]
macro_rules! sound_project {
    (
        $vis:vis $project:ident {
            samples: [
                $($sample:ident => $data:expr),* $(,)?
            ],
            layout: $layout:expr $(,)?
        }
    ) => {
        #[allow(non_camel_case_types)]
        $vis struct $project;

        #[allow(non_camel_case_types)]
        impl $project {
            pub const DATA: $crate::spu::SoundProject<
                { $crate::sound_project!(@count $($sample)*) }
            > = $crate::spu::SoundProject {
                samples: [$($data),*],
                layout: $layout,
            };

            $crate::sound_project!(@ids 0u8, $($sample,)*);
        }
    };

    (@count) => { 0usize };
    (@count $_x:ident $($rest:ident)*) => {
        1usize + $crate::sound_project!(@count $($rest)*)
    };

    (@ids $_idx:expr,) => {};
    (@ids $idx:expr, $name:ident, $($rest:ident,)*) => {
        pub const $name: $crate::spu::SampleId = $crate::spu::SampleId($idx);
        $crate::sound_project!(@ids $idx + 1u8, $($rest,)*);
    };
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
