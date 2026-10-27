//! Publisher-operated public views used only while private cameras are unconfigured.
//! Metadata only: importing this module never connects to a publisher.
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PublicCamera {
    pub id: &'static str,
    pub city: &'static str,
    pub name: &'static str,
    pub publisher: &'static str,
    pub video_id: &'static str,
    pub source_url: &'static str,
}

pub const PUBLIC_CAMERA_FEEDS: &[PublicCamera] = &[
    PublicCamera {
        id: "dc",
        city: "DC",
        name: "White House",
        publisher: "earthTV",
        video_id: "5p_KGD4fJZs",
        source_url: "https://www.youtube.com/watch?v=5p_KGD4fJZs",
    },
    PublicCamera {
        id: "nyc",
        city: "NYC",
        name: "Times Square Crossroads",
        publisher: "EarthCam",
        video_id: "z-jYdOIKcTQ",
        source_url: "https://www.youtube.com/watch?v=z-jYdOIKcTQ",
    },
    PublicCamera {
        id: "boston",
        city: "BOSTON",
        name: "Boston Harbor",
        publisher: "Boston and Maine Live / Hyatt Regency Boston Harbor",
        video_id: "3I_cIS24AfY",
        source_url: "https://www.youtube.com/watch?v=3I_cIS24AfY",
    },
    PublicCamera {
        id: "sf",
        city: "SF",
        name: "San Francisco skyline",
        publisher: "ABC7 News Bay Area",
        video_id: "G8RIAgPxaMc",
        source_url: "https://www.youtube.com/watch?v=G8RIAgPxaMc",
    },
];
