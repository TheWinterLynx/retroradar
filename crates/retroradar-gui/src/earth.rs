//! Earth texture quality is independent of window resolution and orbital state.
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EarthQuality {
    #[default]
    Auto,
    Low,
    Medium,
    High,
}

impl fmt::Display for EarthQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Auto => "auto",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        })
    }
}

impl FromStr for EarthQuality {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err("earth quality must be auto, low, medium or high".into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TexturePlan {
    pub day_width: u32,
    pub night_width: u32,
    pub coast_width: u32,
}

impl TexturePlan {
    pub fn summary(self) -> String {
        format!(
            "DAY {}x{} / NIGHT {}x{} / COAST {}x{}",
            self.day_width,
            self.day_width / 2,
            self.night_width,
            self.night_width / 2,
            self.coast_width,
            self.coast_width / 2,
        )
    }
}

impl EarthQuality {
    pub(crate) fn plan(self, device_max: u32, software: bool) -> Result<TexturePlan, String> {
        if device_max < 2 {
            return Err("GPU cannot hold a 2:1 Earth texture".into());
        }
        // Power-of-two sizes keep the equirectangular 2:1 aspect through the mip chain.
        let cap = 1u32 << device_max.min(if software { 2048 } else { 16384 }).ilog2();
        // Auto is deliberately conservative: dimension limits do not measure free VRAM.
        let requested = match self {
            Self::Low => 4096,
            Self::Auto | Self::Medium => 8192,
            Self::High => 16384,
        };
        let day_width = requested.min(cap);
        Ok(TexturePlan {
            day_width,
            night_width: (day_width / 2).max(2),
            coast_width: day_width.min(8192),
        })
    }
}

pub(crate) fn day_jpeg(width: u32) -> &'static [u8] {
    if width > 8192 {
        crate::assets::EARTH_16K_JPEG
    } else if width > 4096 {
        crate::assets::EARTH_8K_JPEG
    } else {
        crate::assets::EARTH_JPEG
    }
}

pub(crate) fn night_jpeg(width: u32) -> &'static [u8] {
    if width > 4096 {
        crate::assets::NIGHT_8K_JPEG
    } else if width > 2048 {
        crate::assets::NIGHT_4K_JPEG
    } else {
        crate::assets::NIGHT_JPEG
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_uses_real_16k_day_and_8k_night_when_supported() {
        assert_eq!(
            EarthQuality::High.plan(16384, false).unwrap(),
            TexturePlan {
                day_width: 16384,
                night_width: 8192,
                coast_width: 8192,
            }
        );
    }

    #[test]
    fn quality_caps_apply_to_day_night_and_coasts() {
        for max in [2, 512, 2048, 4096, 8191, 8192, 16384, 32768] {
            for quality in [
                EarthQuality::Auto,
                EarthQuality::Low,
                EarthQuality::Medium,
                EarthQuality::High,
            ] {
                for software in [false, true] {
                    let p = quality.plan(max, software).unwrap();
                    for width in [p.day_width, p.night_width, p.coast_width] {
                        assert!(width.is_power_of_two() && width <= max);
                        if software {
                            assert!(width <= 2048);
                        }
                    }
                }
            }
        }
        assert!(EarthQuality::High.plan(1, false).is_err());
        assert_eq!(
            EarthQuality::Auto.plan(16384, false).unwrap().day_width,
            8192
        );
        assert_eq!(
            EarthQuality::High.plan(4096, false).unwrap().night_width,
            2048
        );
    }

    #[test]
    fn quality_names_round_trip_and_invalid_values_fail() {
        for quality in [
            EarthQuality::Auto,
            EarthQuality::Low,
            EarthQuality::Medium,
            EarthQuality::High,
        ] {
            assert_eq!(
                quality.to_string().parse::<EarthQuality>().unwrap(),
                quality
            );
        }
        assert!("unknown".parse::<EarthQuality>().is_err());
    }

    #[test]
    fn bundled_texture_dimensions_match_the_quality_tiers() {
        for (data, size) in [
            (day_jpeg(4096), (4096, 2048)),
            (day_jpeg(8192), (8192, 4096)),
            (day_jpeg(16384), (16384, 8192)),
            (night_jpeg(2048), (2048, 1024)),
            (night_jpeg(4096), (4096, 2048)),
            (night_jpeg(8192), (8192, 4096)),
        ] {
            let reader = image::ImageReader::with_format(
                std::io::Cursor::new(data),
                image::ImageFormat::Jpeg,
            );
            assert_eq!(reader.into_dimensions().unwrap(), size);
        }
    }
    #[test]
    #[ignore = "decodes 16K/8K assets; run explicitly in the HD gate"]
    fn high_source_decodes_without_upscaling() {
        for (data, expected) in [
            (day_jpeg(16384), (16384, 8192)),
            (night_jpeg(8192), (8192, 4096)),
        ] {
            let image = image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
                .unwrap()
                .to_rgba8();
            assert_eq!(image.dimensions(), expected);
            println!("HD DECODE OK: {}x{}", expected.0, expected.1);
        }
    }
}
