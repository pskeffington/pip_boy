//! Bounded same-origin relay for the explicitly opened MWRA public HLS camera.
//! Source origin/path are constants; caller URLs, redirects and proxies are rejected.
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use std::io::{self, Read};
use std::time::Duration;

const BASE: &str = "https://www.mwra.com/sites/default/files/camera-streams/";
const MAX_PLAYLIST: usize = 16 * 1024;
const MAX_SEGMENT: usize = 8 * 1024 * 1024;

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Public camera source unavailable",
    )
}

fn segment_name(name: &str) -> bool {
    name.strip_prefix("ditp")
        .and_then(|v| v.strip_suffix(".ts"))
        .is_some_and(|v| !v.is_empty() && v.len() <= 10 && v.bytes().all(|b| b.is_ascii_digit()))
}

fn rewrite_playlist(text: &str) -> io::Result<Vec<u8>> {
    if !text.starts_with("#EXTM3U") || text.len() > MAX_PLAYLIST {
        return Err(invalid());
    }
    let mut output = String::new();
    let mut segments = 0;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            // This fixed source publishes plain MPEG-TS media playlists.
            // Never forward remote keys, nested playlists or arbitrary URIs.
            if line.contains("URI=") || line.contains("://") || line == "#EXT-X-ENDLIST" {
                return Err(invalid());
            }
            output.push_str(line);
        } else if segment_name(line) {
            segments += 1;
            if segments > 64 {
                return Err(invalid());
            }
            output.push_str("/api/cameras/boston/segment/");
            output.push_str(line);
        } else {
            return Err(invalid());
        }
        output.push('\n');
    }
    if segments == 0 {
        return Err(invalid());
    }
    Ok(output.into_bytes())
}

pub fn fetch(path: &str) -> io::Result<(&'static str, Vec<u8>)> {
    let (filename, playlist, limit) = if path == "/api/cameras/boston/playlist.m3u8" {
        ("ditp.m3u8", true, MAX_PLAYLIST)
    } else {
        let name = path
            .strip_prefix("/api/cameras/boston/segment/")
            .filter(|name| segment_name(name))
            .ok_or_else(invalid)?;
        (name, false, MAX_SEGMENT)
    };
    let client = Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(Policy::none())
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(8))
        .user_agent("Rust-Boy-CAMERA/0.1 (explicit public receiver)")
        .build()
        .map_err(|_| invalid())?;
    let response = client
        .get(format!("{BASE}{filename}"))
        .header("Accept-Encoding", "identity")
        .send()
        .map_err(|_| invalid())?;
    if response.status().as_u16() != 200 {
        return Err(invalid());
    }
    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim();
    let type_allowed = if playlist {
        ["application/vnd.apple.mpegurl", "application/x-mpegurl"].contains(&mime)
    } else {
        mime == "video/mp2t"
    };
    let oversized = response
        .content_length()
        .is_some_and(|size| size > limit as u64);
    if !type_allowed || oversized {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    response.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(invalid());
    }
    if playlist {
        let text = std::str::from_utf8(&bytes).map_err(|_| invalid())?;
        Ok(("application/vnd.apple.mpegurl", rewrite_playlist(text)?))
    } else if bytes.len() >= 564 && [0, 188, 376].iter().all(|&index| bytes[index] == 0x47) {
        Ok(("video/mp2t", bytes))
    } else {
        Err(invalid())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_fixed_camera_transport_names_are_allowed() {
        assert!(segment_name("ditp123.ts"));
        for name in [
            "../ditp1.ts",
            "https://other/ditp1.ts",
            "ditp1.ts?url=x",
            "cosgrove1.ts",
            "ditp.ts",
        ] {
            assert!(!segment_name(name));
        }
    }

    #[test]
    fn playlist_rewrite_cannot_become_an_arbitrary_url_proxy() {
        let safe =
            rewrite_playlist("#EXTM3U\n#EXT-X-TARGETDURATION:9\n#EXTINF:8,\nditp31.ts\n").unwrap();
        assert!(String::from_utf8(safe)
            .unwrap()
            .contains("/api/cameras/boston/segment/ditp31.ts"));
        for unsafe_list in [
            "#EXTM3U\nhttps://example.com/video.ts\n",
            "#EXTM3U\n../ditp1.ts\n",
            "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"secret.key\"\nditp1.ts\n",
            "#EXTM3U\nditp1.ts\n#EXT-X-ENDLIST\n",
        ] {
            assert!(rewrite_playlist(unsafe_list).is_err());
        }
    }
}
