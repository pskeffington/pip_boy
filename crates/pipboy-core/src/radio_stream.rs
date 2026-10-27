//! Host compatibility path for the standalone rust_radio module.
pub use rust_radio::stream::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};

    struct Fixture {
        url: &'static str,
        status: u16,
        mime: &'static str,
        redirect: Option<&'static str>,
        metaint: Option<&'static str>,
        body: Vec<u8>,
    }

    struct FakeHttp(Vec<Fixture>);

    impl RadioHttp for FakeHttp {
        type Body = Cursor<Vec<u8>>;

        fn get(&self, target: &ApprovedEndpoint) -> Result<HttpResponse<Self::Body>, StreamError> {
            let fixture = self
                .0
                .iter()
                .find(|entry| entry.url == target.url())
                .ok_or(StreamError::TransportUnavailable)?;
            Ok(HttpResponse {
                status: fixture.status,
                content_type: Some(fixture.mime.into()),
                redirect_location: fixture.redirect.map(str::to_string),
                icy_metaint: fixture.metaint.map(str::to_string),
                body: Cursor::new(fixture.body.clone()),
            })
        }
    }

    #[test]
    fn tls_and_station_origin_are_enforced() {
        for url in [
            "http://somafm.com/m3u/groovesalad.m3u",
            "https://127.0.0.1/audio",
            "https://somafm.com.evil.test/audio",
            "https://evil.test@somafm.com/stream",
            "https://somafm.com:8080/audio",
            "https://somafm.com/%2e%2e/secret",
            "https://somafm.com/../secret",
            "https://somafm.com/stream?redirect=localhost",
            "https://uksoutha.streaming.broadcast.radio:29690/skylab-radio-limited",
        ] {
            assert!(ApprovedEndpoint::new("groovesalad", url).is_err(), "{url}");
        }
        assert!(ApprovedEndpoint::new(
            "skylab",
            "https://uksoutha.streaming.broadcast.radio:29690/skylab-radio-limited"
        )
        .is_ok());
        assert!(ApprovedEndpoint::new(
            "groovesalad",
            "https://ice5.somafm.com/groovesalad-128-mp3"
        )
        .is_ok());
    }

    #[test]
    fn m3u_limits_and_comments_are_handled() {
        let playlist = b"#EXTM3U\r\n#EXTINF:-1,Groove Salad\r\nhttps://ice5.somafm.com/groovesalad-128-mp3\r\n";
        let parsed = parse_m3u("groovesalad", playlist).unwrap();
        assert_eq!(parsed.len(), 1);
        assert!(parsed[0].url().contains("ice5"));
        assert!(matches!(
            parse_m3u("groovesalad", &vec![b'a'; MAX_PLAYLIST_BYTES + 1]),
            Err(StreamError::OversizedPlaylist)
        ));
        assert!(parse_m3u("groovesalad", b"#EXTM3U\n# comment").is_err());
        assert!(parse_m3u("groovesalad", b"https://example.com/steal").is_err());
    }

    #[test]
    fn playlist_resolves_only_to_allowed_audio_response() {
        let station_url = "https://somafm.com/m3u/groovesalad.m3u";
        let ice = "https://ice5.somafm.com/groovesalad-128-mp3";
        let fixture = FakeHttp(vec![
            Fixture {
                url: station_url,
                status: 200,
                mime: "audio/x-mpegurl",
                redirect: None,
                metaint: None,
                body: format!("#EXTM3U\n{ice}\n").into_bytes(),
            },
            Fixture {
                url: ice,
                status: 200,
                mime: "audio/mpeg",
                redirect: None,
                metaint: Some("4"),
                body: {
                    let mut body = b"abcd".to_vec();
                    let mut meta = [0u8; 16];
                    meta.copy_from_slice(b"StreamTitle='X';");
                    body.push(1);
                    body.extend_from_slice(&meta);
                    body.extend_from_slice(b"efgh");
                    body
                },
            },
        ]);
        let mut opened = open_station(&fixture, "groovesalad").unwrap();
        assert_eq!(opened.endpoint.url(), ice);
        let mut audio = [0u8; 4];
        assert_eq!(opened.reader.read(&mut audio).unwrap(), 4);
        assert_eq!(&audio, b"abcd");
        assert_eq!(opened.reader.read(&mut audio).unwrap(), 4);
        assert_eq!(&audio, b"efgh");
        assert_eq!(opened.reader.take_last_title(), Some("X".into()));
    }

    #[test]
    fn disallowed_redirect_cannot_be_requested() {
        let fixture = FakeHttp(vec![Fixture {
            url: "https://somafm.com/m3u/groovesalad.m3u",
            status: 302,
            mime: "text/plain",
            redirect: Some("https://127.0.0.1/internal"),
            metaint: None,
            body: vec![],
        }]);
        assert!(matches!(
            open_station(&fixture, "groovesalad"),
            Err(StreamError::UnsafeEndpoint)
        ));
    }

    #[test]
    fn invalid_icy_interval_and_content_are_rejected() {
        assert!(matches!(
            icy_interval(Some("0")),
            Err(StreamError::InvalidIcyInterval)
        ));
        assert!(matches!(
            icy_interval(Some("abc")),
            Err(StreamError::InvalidIcyInterval)
        ));
        let html = FakeHttp(vec![
            Fixture {
                url: "https://somafm.com/m3u/groovesalad.m3u",
                status: 302,
                mime: "text/plain",
                redirect: Some("https://ice5.somafm.com/groovesalad-128-mp3"),
                metaint: None,
                body: vec![],
            },
            Fixture {
                url: "https://ice5.somafm.com/groovesalad-128-mp3",
                status: 200,
                mime: "text/html",
                redirect: None,
                metaint: None,
                body: b"<html>not audio</html>".to_vec(),
            },
        ]);
        assert!(matches!(
            open_station(&html, "groovesalad"),
            Err(StreamError::UnsupportedContentType)
        ));
    }

    #[test]
    fn truncated_metadata_is_a_stream_error() {
        let data = b"ABCD\x02too-short".to_vec();
        let mut reader = IcyAudioReader::new(Cursor::new(data), Some(4));
        let mut buf = [0u8; 4];
        assert_eq!(reader.read(&mut buf).unwrap(), 4);
        assert!(reader.read(&mut buf).is_err());
    }
}
