//! Direct publisher media catalog. Metadata alone never opens a connection.
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PublicCamera {
    pub id: &'static str,
    pub city: &'static str,
    pub name: &'static str,
    pub publisher: &'static str,
    pub kind: &'static str,
    pub media_url: &'static str,
    pub source_url: &'static str,
    pub refresh_seconds: u16,
}

pub const PUBLIC_CAMERA_FEEDS: &[PublicCamera] = &[
    PublicCamera {
        id: "dc",
        city: "DC",
        name: "Direct feed pending validation",
        publisher: "Public Washington camera",
        kind: "unavailable",
        media_url: "",
        source_url: "https://www.earthcam.com/usa/dc/washingtonmonument/",
        refresh_seconds: 0,
    },
    PublicCamera {
        id: "nyc",
        city: "NYC",
        name: "NYC DOT source unavailable at review",
        publisher: "NYC DOT",
        kind: "unavailable",
        media_url: "",
        source_url: "https://www.nyc.gov/html/dot/html/motorist/atis.shtml",
        refresh_seconds: 0,
    },
    PublicCamera {
        id: "boston",
        city: "BOSTON",
        name: "Deer Island / Boston Harbor",
        publisher: "Massachusetts Water Resources Authority",
        kind: "hls",
        media_url: "/api/cameras/boston/playlist.m3u8",
        source_url: "https://www.mwra.com/about-mwra/live-camera-feeds",
        refresh_seconds: 0,
    },
    PublicCamera {
        id: "sf",
        city: "SF",
        name: "FogCam / San Francisco State University",
        publisher: "FogCam / SFSU",
        kind: "jpeg",
        media_url: "https://fogcam.org/fogcam2.jpg",
        source_url: "https://fogcam.org/",
        refresh_seconds: 20,
    },
];
