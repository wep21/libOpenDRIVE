use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OpenDriveError {
    #[error("File not found: {0}")]
    IoError(#[from] std::io::Error),
    #[error("XML parsing error: {0}")]
    XmlError(#[from] quick_xml::DeError),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct OpenDrive {
    #[serde(rename = "road", default)]
    pub roads: Vec<Road>,
    #[serde(rename = "junction", default)]
    pub junctions: Vec<Junction>,
}

impl OpenDrive {
    pub fn new(path: &str) -> Result<Self, OpenDriveError> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let open_drive: OpenDrive = quick_xml::de::from_str(&contents)?;
        Ok(open_drive)
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Road {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@length")]
    pub length: f64,
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@junction")]
    pub junction: String,
    #[serde(rename = "link")]
    pub link: Option<Link>,
    #[serde(rename = "type", default)]
    pub road_type: Vec<RoadType>,
    #[serde(rename = "planView")]
    pub plan_view: PlanView,
    #[serde(rename = "elevationProfile")]
    pub elevation_profile: Option<ElevationProfile>,
    #[serde(rename = "lateralProfile")]
    pub lateral_profile: Option<LateralProfile>,
    pub lanes: Lanes,
}

use fresnel::fresnl;
use std::f64::consts::PI;

impl Road {
    pub fn get_point(&self, s: f64, _t: f64, _h: f64) -> (f64, f64, f64, f64) {
        let mut x = 0.0;
        let mut y = 0.0;
        let z = 0.0;
        let mut heading = 0.0;

        let mut current_s = 0.0;
        for geom in &self.plan_view.geometry {
            if s >= current_s && s < current_s + geom.length + 1e-6 {
                let ds = s - current_s;
                match &geom.type_specific {
                    GeometryType::Line(_) => {
                        x = geom.x + ds * geom.hdg.cos();
                        y = geom.y + ds * geom.hdg.sin();
                        heading = geom.hdg;
                    }
                    GeometryType::Arc(arc) => {
                        let c = arc.curvature;
                        x = geom.x + (1.0 / c) * ((geom.hdg + c * ds).sin() - geom.hdg.sin());
                        y = geom.y + (1.0 / c) * (geom.hdg.cos() - (geom.hdg + c * ds).cos());
                        heading = geom.hdg + ds * c;
                    }
                    GeometryType::Spiral(spiral) => {
                        let c_dot = (spiral.curv_end - spiral.curv_start) / geom.length;
                        let a = c_dot.abs().sqrt() / PI.sqrt();
                        let (s_fresnel, c_fresnel) = fresnl(a * ds);
                        let factor = PI.sqrt() / (2.0 * a);
                        x = geom.x + factor * (c_fresnel * geom.hdg.cos() - s_fresnel * geom.hdg.sin());
                        y = geom.y + factor * (c_fresnel * geom.hdg.sin() + s_fresnel * geom.hdg.cos());
                        heading = geom.hdg + spiral.curv_start * ds + c_dot * ds * ds / 2.0;
                    }
                }
                break;
            }
            current_s += geom.length;
        }

        (x, y, z, heading)
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Junction {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(default)]
    pub connection: Vec<Connection>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Connection {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@incomingRoad")]
    pub incoming_road: String,
    #[serde(rename = "@connectingRoad")]
    pub connecting_road: String,
    #[serde(rename = "@contactPoint")]
    pub contact_point: String,
    #[serde(rename = "laneLink", default)]
    pub lane_links: Vec<JunctionLaneLink>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct JunctionLaneLink {
    #[serde(rename = "@from")]
    pub from: i32,
    #[serde(rename = "@to")]
    pub to: i32,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Link {
    pub predecessor: Option<Predecessor>,
    pub successor: Option<Successor>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Predecessor {
    #[serde(rename = "@elementType", default)]
    pub element_type: String,
    #[serde(rename = "@elementId", default)]
    pub element_id: String,
    #[serde(rename = "@contactPoint", default)]
    pub contact_point: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Successor {
    #[serde(rename = "@elementType", default)]
    pub element_type: String,
    #[serde(rename = "@elementId", default)]
    pub element_id: String,
    #[serde(rename = "@contactPoint", default)]
    pub contact_point: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Speed {
    #[serde(rename = "@max")]
    pub max: String,
    #[serde(rename = "@unit")]
    pub unit: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct RoadType {
    #[serde(rename = "@s")]
    pub s: f64,
    #[serde(rename = "@type")]
    pub type_name: String,
    pub speed: Option<Speed>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct PlanView {
    #[serde(rename = "$value", default)]
    pub geometry: Vec<Geometry>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Geometry {
    #[serde(rename = "@s")]
    pub s: f64,
    #[serde(rename = "@x")]
    pub x: f64,
    #[serde(rename = "@y")]
    pub y: f64,
    #[serde(rename = "@hdg")]
    pub hdg: f64,
    #[serde(rename = "@length")]
    pub length: f64,
    #[serde(rename = "$value")]
    pub type_specific: GeometryType,
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum GeometryType {
    #[serde(rename = "line")]
    Line(Line),
    #[serde(rename = "spiral")]
    Spiral(Spiral),
    #[serde(rename = "arc")]
    Arc(Arc),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Line {}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Spiral {
    #[serde(rename = "@curvStart", default)]
    pub curv_start: f64,
    #[serde(rename = "@curvEnd", default)]
    pub curv_end: f64,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Arc {
    #[serde(rename = "@curvature", default)]
    pub curvature: f64,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ElevationProfile {
    #[serde(rename = "elevation", default)]
    pub elevation: Vec<Elevation>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Elevation {
    #[serde(rename = "@s")]
    pub s: f64,
    #[serde(rename = "@a")]
    pub a: f64,
    #[serde(rename = "@b")]
    pub b: f64,
    #[serde(rename = "@c")]
    pub c: f64,
    #[serde(rename = "@d")]
    pub d: f64,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct LateralProfile {
    #[serde(rename = "superelevation", default)]
    pub superelevation: Vec<Superelevation>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Superelevation {
    #[serde(rename = "@s")]
    pub s: f64,
    #[serde(rename = "@a")]
    pub a: f64,
    #[serde(rename = "@b")]
    pub b: f64,
    #[serde(rename = "@c")]
    pub c: f64,
    #[serde(rename = "@d")]
    pub d: f64,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Lanes {
    #[serde(rename = "laneOffset", default)]
    pub lane_offset: Vec<LaneOffset>,
    #[serde(rename = "laneSection", default)]
    pub lane_section: Vec<LaneSection>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct LaneOffset {
    #[serde(rename = "@s")]
    pub s: f64,
    #[serde(rename = "@a")]
    pub a: f64,
    #[serde(rename = "@b")]
    pub b: f64,
    #[serde(rename = "@c")]
    pub c: f64,
    #[serde(rename = "@d")]
    pub d: f64,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct LaneSection {
    #[serde(rename = "@s")]
    pub s: f64,
    pub left: Option<Left>,
    pub center: Center,
    pub right: Option<Right>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_test_xodr() {
        let xml_content = include_str!("../../tests/test.xodr");
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "{}", xml_content).unwrap();
        let open_drive = OpenDrive::new(file.path().to_str().unwrap()).unwrap();
        assert_eq!(open_drive.roads.len(), 234);
    }

    #[test]
    fn test_get_point_line() {
        let xml_content = include_str!("../../tests/simple_line.xodr");
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "{}", xml_content).unwrap();
        let open_drive = OpenDrive::new(file.path().to_str().unwrap()).unwrap();
        let road = &open_drive.roads[0];
        let (x, y, z, heading) = road.get_point(10.0, 0.0, 0.0);
        assert!((x - 10.0).abs() < 1e-6);
        assert!((y - 0.0).abs() < 1e-6);
        assert!((z - 0.0).abs() < 1e-6);
        assert!((heading - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_get_point_arc() {
        let xml_content = include_str!("../../tests/simple_arc.xodr");
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "{}", xml_content).unwrap();
        let open_drive = OpenDrive::new(file.path().to_str().unwrap()).unwrap();
        let road = &open_drive.roads[0];
        let (x, y, z, heading) = road.get_point(15.7079632679, 0.0, 0.0);
        println!("x: {}, y: {}", x, y);
        assert!((x - 10.0).abs() < 1e-6);
        assert!((y - 10.0).abs() < 1e-6);
        assert!((z - 0.0).abs() < 1e-6);
        assert!((heading - 1.57079632679).abs() < 1e-6);
    }

    #[test]
    fn test_get_point_spiral() {
        let xml_content = include_str!("../../tests/simple_spiral.xodr");
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "{}", xml_content).unwrap();
        let open_drive = OpenDrive::new(file.path().to_str().unwrap()).unwrap();
        let road = &open_drive.roads[0];
        let (x, y, z, heading) = road.get_point(100.0, 0.0, 0.0);
        // The expected values are calculated from an external tool.
        assert!((x - 16.3154).abs() < 1e-4);
        assert!((y - 23.1447).abs() < 1e-4);
        assert!((z - 0.0).abs() < 1e-6);
        assert!((heading - 5.0).abs() < 1e-4);
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Left {
    #[serde(rename = "$value", default)]
    pub lane: Vec<Lane>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Center {
    pub lane: Lane,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Right {
    #[serde(rename = "$value", default)]
    pub lane: Vec<Lane>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Lane {
    #[serde(rename = "@id")]
    pub id: i32,
    #[serde(rename = "@type")]
    pub lane_type: String,
    #[serde(rename = "@level")]
    pub level: String,
    pub link: Option<LaneLink>,
    #[serde(rename = "width", default)]
    pub width: Vec<LaneWidth>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct LaneLink {
    pub predecessor: Option<LanePredecessor>,
    pub successor: Option<LaneSuccessor>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct LanePredecessor {
    #[serde(rename = "@id")]
    pub id: i32,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct LaneSuccessor {
    #[serde(rename = "@id")]
    pub id: i32,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct LaneWidth {
    #[serde(rename = "@sOffset")]
    pub s_offset: f64,
    #[serde(rename = "@a")]
    pub a: f64,
    #[serde(rename = "@b")]
    pub b: f64,
    #[serde(rename = "@c")]
    pub c: f64,
    #[serde(rename = "@d")]
    pub d: f64,
}

#[cfg(feature = "ffi")]
pub mod ffi {
    use super::*;
    use std::ffi::CStr;
    use std::os::raw::c_char;

    #[no_mangle]
    pub extern "C" fn opendrive_load(path: *const c_char) -> *mut OpenDrive {
        let c_str = unsafe {
            assert!(!path.is_null());
            CStr::from_ptr(path)
        };
        let r_str = c_str.to_str().unwrap();
        match OpenDrive::new(r_str) {
            Ok(open_drive) => Box::into_raw(Box::new(open_drive)),
            Err(_) => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub extern "C" fn opendrive_free(ptr: *mut OpenDrive) {
        if ptr.is_null() {
            return;
        }
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }

    #[repr(C)]
    pub struct Point {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub heading: f64,
    }

    #[no_mangle]
    pub extern "C" fn opendrive_get_road_count(ptr: *const OpenDrive) -> usize {
        let open_drive = unsafe {
            assert!(!ptr.is_null());
            &*ptr
        };
        open_drive.roads.len()
    }

    #[no_mangle]
    pub extern "C" fn opendrive_get_road(ptr: *const OpenDrive, index: usize) -> *const Road {
        let open_drive = unsafe {
            assert!(!ptr.is_null());
            &*ptr
        };
        if index < open_drive.roads.len() {
            &open_drive.roads[index] as *const Road
        } else {
            std::ptr::null()
        }
    }

    #[no_mangle]
    pub extern "C" fn road_get_point(ptr: *const Road, s: f64, t: f64, h: f64) -> Point {
        let road = unsafe {
            assert!(!ptr.is_null());
            &*ptr
        };
        let (x, y, z, heading) = road.get_point(s, t, h);
        Point { x, y, z, heading }
    }
}
