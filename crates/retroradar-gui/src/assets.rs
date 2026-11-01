//! Embedded assets (licences in `assets/LICENSES.md`).

pub const EARTH_JPEG: &[u8] = include_bytes!("../../../assets/earth/blue_marble_4096x2048.jpg");
pub const EARTH_8K_JPEG: &[u8] = include_bytes!("../../../assets/earth/blue_marble_8192x4096.jpg");
pub const EARTH_16K_JPEG: &[u8] =
    include_bytes!("../../../assets/earth/blue_marble_16384x8192.jpg");
pub const NIGHT_4K_JPEG: &[u8] = include_bytes!("../../../assets/earth/black_marble_4096x2048.jpg");
pub const NIGHT_8K_JPEG: &[u8] = include_bytes!("../../../assets/earth/black_marble_8192x4096.jpg");
pub const NIGHT_JPEG: &[u8] = include_bytes!("../../../assets/earth/black_marble_2048x1024.jpg");
pub const COAST_110M: &[u8] = include_bytes!("../../../assets/coastlines/ne_110m_coastline.bin");
pub const COAST_50M: &[u8] = include_bytes!("../../../assets/coastlines/ne_50m_coastline.bin");
pub const FONT_JETBRAINS: &[u8] = include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf");
pub const FONT_UNSCII: &[u8] = include_bytes!("../../../assets/fonts/unscii-16.ttf");
