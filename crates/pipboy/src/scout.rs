//! Scout GIS commands: explicit local-only GeoJSON I/O and spherical distance.
//! No live positioning, RustGIS embedding, TAK network connections or location logging.
use pipboy_core::scout_gis::{distance_meters, inspect_geojson, waypoint, Point, MAX_GEOJSON_BYTES};
use std::error::Error;
use std::fs::File;
use std::io::{self, Read};

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let usage = "usage: pipboy scout geo (waypoint NAME LAT LON | distance LAT1 LON1 LAT2 LON2 | inspect GEOJSON_FILE)";
    let mut args = args.into_iter();
    if args.next().as_deref() != Some("geo") {
        return Err(usage.into());
    }
    match args.next().as_deref() {
        Some("waypoint") => {
            let name = args.next().ok_or(usage)?;
            let lat = args.next().ok_or(usage)?;
            let lon = args.next().ok_or(usage)?;
            if args.next().is_some() {
                return Err(usage.into());
            }
            let point = Point::from_text(&lat, &lon)?;
            let output = waypoint(&name, point)?;
            println!("{}", serde_json::to_string_pretty(&output)?);
            Ok(())
        }
        Some("distance") => {
            let lat_a = args.next().ok_or(usage)?;
            let lon_a = args.next().ok_or(usage)?;
            let lat_b = args.next().ok_or(usage)?;
            let lon_b = args.next().ok_or(usage)?;
            if args.next().is_some() {
                return Err(usage.into());
            }
            let a = Point::from_text(&lat_a, &lon_a)?;
            let b = Point::from_text(&lat_b, &lon_b)?;
            println!(
                "SCOUT GIS / great-circle spherical estimate\nDistance: {:.2} m ({:.3} km)\nNo network or GNSS used.",
                distance_meters(a, b),
                distance_meters(a, b) / 1000.0
            );
            Ok(())
        }
        Some("inspect") => {
            let path = args.next().ok_or(usage)?;
            if args.next().is_some() {
                return Err(usage.into());
            }
            let file = File::open(&path)?;
            let metadata = file.metadata()?;
            if !metadata.is_file() || metadata.len() > MAX_GEOJSON_BYTES as u64 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "expected a regular GeoJSON file no larger than 1 MiB",
                )
                .into());
            }
            let mut data = Vec::new();
            file.take((MAX_GEOJSON_BYTES + 1) as u64)
                .read_to_end(&mut data)?;
            let result = inspect_geojson(&data)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(())
        }
        _ => Err(usage.into()),
    }
}
