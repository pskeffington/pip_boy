//! Curated, publisher-operated transport camera snapshots.
//! No YouTube, discovery, credentials, recording, or implicit network requests.
//! Sources are listed in public agency feeds; per-device playback is not certified.
use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct PublicCamera {
    pub id: &'static str,
    pub city: &'static str,
    pub name: &'static str,
    pub publisher: &'static str,
    pub source_url: &'static str,
    pub image_url: &'static str,
    pub region: &'static str,
    pub lat: f64,
    pub lon: f64,
    pub format: &'static str,
    pub validation: &'static str,
}
pub const PUBLIC_CAMERA_FEEDS: &[PublicCamera] = &[
    PublicCamera {
        id:"dc",city:"DC METRO",name:"US-50 / Patrick Henry Drive (Arlington, VA)",
        publisher:"VDOT / Virginia 511",source_url:"https://511.vdot.virginia.gov/",
        image_url:"https://snapshot.vdotcameras.com/thumbs/0wyvj6n16r826rz758qn5mf48ogpj1fp.flv.png",
        region:"VA",lat:38.868556,lon:-77.14676,format:"jpeg_or_png_snapshot",
        validation:"published_in_agency_feed_unverified_live",
    },
    PublicCamera {
        id:"nyc",city:"NYC",name:"NYC DOT camera access pending",
        publisher:"NYC DOT TMC",source_url:"https://webcams.nyctmc.org/subscribers",
        image_url:"",region:"NY",lat:40.7580,lon:-73.9855,format:"unavailable",
        validation:"developer_agreement_required",
    },
    PublicCamera {
        id:"boston",city:"BOSTON",name:"MassDOT / TrafficLand access pending",
        publisher:"MassDOT",source_url:"https://www.mass.gov/info-details/highway-data-for-developers",
        image_url:"",region:"MA",lat:42.3601,lon:-71.0589,format:"unavailable",
        validation:"provider_access_required",
    },
    PublicCamera {
        id:"sf",city:"SF",name:"US-101 at Octavia Street (San Francisco)",
        publisher:"Caltrans D04",source_url:"https://cwwp2.dot.ca.gov/documentation/cctv/cctv.htm",
        image_url:"https://cwwp2.dot.ca.gov/data/d4/cctv/image/tv301us101atoctaviast/tv301us101atoctaviast.jpg",
        region:"CA",lat:37.775,lon:-122.424,format:"jpeg_snapshot",
        validation:"published_snapshot_url_unverified_live",
    },
];
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn public_camera_urls_are_dot_and_not_youtube() {
        assert_eq!(PUBLIC_CAMERA_FEEDS.len(),4);
        for feed in PUBLIC_CAMERA_FEEDS {
            assert!(feed.source_url.starts_with("https://"));
            assert!(!feed.source_url.contains("youtube"));
            assert!(!feed.image_url.contains("youtube"));
            if !feed.image_url.is_empty() {
                assert!(feed.image_url.starts_with("https://"));
                assert!(feed.image_url.starts_with("https://snapshot.vdotcameras.com/thumbs/") ||
                        feed.image_url.starts_with("https://cwwp2.dot.ca.gov/data/d4/cctv/image/"));
            }
        }
    }
}
