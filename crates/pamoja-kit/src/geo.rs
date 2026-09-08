//! Working with positions on the Earth: distance and staying inside an area.

use core::f64::consts::PI;

use libm::{atan2, cos, sin, sqrt};

// IUGG mean Earth radius in meters. Treating the Earth as a sphere is accurate to a
// few tenths of a percent, which is well inside the error of a low-cost GPS fix.
const EARTH_RADIUS_M: f64 = 6_371_008.8;

fn to_radians(degrees: f64) -> f64 {
    degrees * (PI / 180.0)
}

fn to_degrees(radians: f64) -> f64 {
    radians * (180.0 / PI)
}

// `f64::abs` lives in `std`, so this `no_std` crate takes the magnitude by hand.
fn magnitude(value: f64) -> f64 {
    if value < 0.0 {
        -value
    } else {
        value
    }
}

/// A position on the Earth, in decimal degrees.
///
/// Latitude and longitude are kept as `f64` because a GPS fix needs more precision
/// than `f32` can hold: rounding a coordinate to `f32` can move it tens of meters.
///
/// # Examples
///
/// Great-circle distance between two cities, in kilometers:
///
/// ```
/// use pamoja_kit::Coordinate;
///
/// let nairobi = Coordinate::new(-1.2921, 36.8219);
/// let mombasa = Coordinate::new(-4.0435, 39.6682);
/// let km = nairobi.distance_to(mombasa) / 1000.0;
/// assert!((km - 440.0).abs() < 10.0); // about 440 km apart
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coordinate {
    /// Degrees north of the equator, in `[-90.0, 90.0]`.
    pub latitude: f64,
    /// Degrees east of the prime meridian, in `[-180.0, 180.0]`.
    pub longitude: f64,
}

impl Coordinate {
    /// Creates a coordinate from a latitude and longitude in decimal degrees.
    ///
    /// # Arguments
    ///
    /// * `latitude` - degrees north of the equator.
    /// * `longitude` - degrees east of the prime meridian.
    ///
    /// # Returns
    ///
    /// The coordinate.
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }

    /// Returns the distance to another coordinate in meters.
    ///
    /// This is the great-circle distance: the shortest path over the surface of a
    /// spherical Earth. The technique one layer down is the haversine formula, which
    /// stays numerically stable for the short distances a field deployment cares
    /// about, down to points a few meters apart.
    ///
    /// # Arguments
    ///
    /// * `other` - the coordinate to measure to.
    ///
    /// # Returns
    ///
    /// The distance in meters, always zero or positive.
    pub fn distance_to(&self, other: Coordinate) -> f64 {
        let lat1 = to_radians(self.latitude);
        let lat2 = to_radians(other.latitude);
        let half_dlat = to_radians(other.latitude - self.latitude) / 2.0;
        let half_dlon = to_radians(other.longitude - self.longitude) / 2.0;
        let sin_lat = sin(half_dlat);
        let sin_lon = sin(half_dlon);
        let a = sin_lat * sin_lat + cos(lat1) * cos(lat2) * sin_lon * sin_lon;
        let c = 2.0 * atan2(sqrt(a), sqrt(1.0 - a));
        EARTH_RADIUS_M * c
    }

    /// Returns the initial bearing to another coordinate, in degrees clockwise from north.
    ///
    /// This is the forward azimuth of the great-circle path: the compass heading to set off
    /// on to reach `other` by the shortest route. Because a great circle curves, the bearing
    /// changes along the way; this is the heading at the start. The result is normalized to
    /// `[0.0, 360.0)`, with 0 north, 90 east, 180 south, and 270 west.
    ///
    /// # Arguments
    ///
    /// * `other` - the coordinate to head toward.
    ///
    /// # Returns
    ///
    /// The initial bearing in degrees, in `[0.0, 360.0)`. When both points are the same the
    /// result is `0.0`.
    pub fn bearing_to(&self, other: Coordinate) -> f64 {
        let lat1 = to_radians(self.latitude);
        let lat2 = to_radians(other.latitude);
        let dlon = to_radians(other.longitude - self.longitude);
        let y = sin(dlon) * cos(lat2);
        let x = cos(lat1) * sin(lat2) - sin(lat1) * cos(lat2) * cos(dlon);
        let bearing = to_degrees(atan2(y, x));
        if bearing < 0.0 {
            bearing + 360.0
        } else {
            bearing
        }
    }
}

/// Where a fix sits relative to a [`Geofence`], including the moment it crosses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    /// The fix is inside the fence and was inside before, or is the first fix inside.
    Inside,
    /// The fix is outside the fence and was outside before, or is the first fix outside.
    Outside,
    /// The fix just crossed from inside to outside: the moment to raise a breach alert.
    Exited,
    /// The fix just crossed from outside back inside.
    Entered,
}

/// Keeping a tracked point inside an area, and noticing when it leaves.
///
/// This is the primitive behind "tell me when it leaves the safe zone": a collared
/// animal straying from its pasture, an asset moving off-site, or a drone crossing
/// its allowed boundary. A fence is a center and a radius; feeding it successive
/// fixes reports whether each is [`Inside`](Boundary::Inside) or
/// [`Outside`](Boundary::Outside) and, crucially, the single fix that
/// [`Exited`](Boundary::Exited) or [`Entered`](Boundary::Entered), so an alert fires
/// once on the crossing rather than on every fix while away.
///
/// # Examples
///
/// ```
/// use pamoja_kit::{Boundary, Coordinate, Geofence};
///
/// // A 50 m pen around the waterpoint; the collar fix then wanders out.
/// let mut pen = Geofence::new(Coordinate::new(-1.2921, 36.8219), 50.0);
/// assert_eq!(pen.update(Coordinate::new(-1.2921, 36.8219)), Boundary::Inside);
/// assert_eq!(pen.update(Coordinate::new(-1.2930, 36.8219)), Boundary::Exited);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geofence {
    center: Coordinate,
    radius_m: f64,
    inside: Option<bool>,
}

impl Geofence {
    /// Creates a fence of `radius_m` meters around `center`.
    ///
    /// # Arguments
    ///
    /// * `center` - the middle of the safe area.
    /// * `radius_m` - the radius of the safe area in meters; its magnitude is used.
    ///
    /// # Returns
    ///
    /// A fence that has not yet seen a fix.
    pub fn new(center: Coordinate, radius_m: f64) -> Self {
        Self {
            center,
            radius_m: magnitude(radius_m),
            inside: None,
        }
    }

    /// Returns whether a point lies within the fence.
    ///
    /// # Arguments
    ///
    /// * `point` - the coordinate to test.
    ///
    /// # Returns
    ///
    /// `true` if `point` is on or inside the fence boundary.
    pub fn contains(&self, point: Coordinate) -> bool {
        self.center.distance_to(point) <= self.radius_m
    }

    /// Records a fix and reports its position relative to the fence.
    ///
    /// # Arguments
    ///
    /// * `point` - the latest fix.
    ///
    /// # Returns
    ///
    /// [`Boundary::Entered`] or [`Boundary::Exited`] on the fix that crosses the
    /// boundary, otherwise [`Boundary::Inside`] or [`Boundary::Outside`].
    pub fn update(&mut self, point: Coordinate) -> Boundary {
        let now_inside = self.contains(point);
        let boundary = match self.inside {
            Some(true) if !now_inside => Boundary::Exited,
            Some(false) if now_inside => Boundary::Entered,
            _ if now_inside => Boundary::Inside,
            _ => Boundary::Outside,
        };
        self.inside = Some(now_inside);
        boundary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_to_self_is_zero() {
        let point = Coordinate::new(12.34, -56.78);
        assert_eq!(point.distance_to(point), 0.0);
    }

    #[test]
    fn one_degree_of_longitude_at_the_equator() {
        // A degree of longitude at the equator is about 111.2 km.
        let here = Coordinate::new(0.0, 0.0);
        let east = Coordinate::new(0.0, 1.0);
        let meters = here.distance_to(east);
        assert!((meters - 111_195.0).abs() < 5.0);
    }

    #[test]
    fn distance_is_symmetric() {
        let a = Coordinate::new(40.7128, -74.0060);
        let b = Coordinate::new(51.5074, -0.1278);
        assert!((a.distance_to(b) - b.distance_to(a)).abs() < 1.0);
    }

    #[test]
    fn an_intercontinental_distance_is_accurate() {
        // New York to London is about 5570 km along the great circle.
        let nyc = Coordinate::new(40.7128, -74.0060);
        let london = Coordinate::new(51.5074, -0.1278);
        let km = nyc.distance_to(london) / 1000.0;
        assert!((km - 5570.0).abs() < 30.0);
    }

    #[test]
    fn antipodal_points_are_half_the_circumference() {
        // Opposite points are pi * R apart, about 20015 km.
        let here = Coordinate::new(0.0, 0.0);
        let opposite = Coordinate::new(0.0, 180.0);
        let km = here.distance_to(opposite) / 1000.0;
        assert!((km - 20_015.0).abs() < 5.0);
    }

    #[test]
    fn bearing_to_the_cardinal_directions() {
        let here = Coordinate::new(0.0, 0.0);
        assert!((here.bearing_to(Coordinate::new(1.0, 0.0)) - 0.0).abs() < 1e-6); // north
        assert!((here.bearing_to(Coordinate::new(0.0, 1.0)) - 90.0).abs() < 1e-6); // east
        let north = Coordinate::new(1.0, 0.0);
        assert!((north.bearing_to(here) - 180.0).abs() < 1e-6); // south
        let east = Coordinate::new(0.0, 1.0);
        assert!((east.bearing_to(here) - 270.0).abs() < 1e-6); // west
    }

    #[test]
    fn bearing_matches_a_worked_example() {
        // Movable Type's worked example: Baghdad (35 N, 45 E) to Osaka (35 N, 135 E)
        // sets off on an initial bearing of about 60 degrees.
        let baghdad = Coordinate::new(35.0, 45.0);
        let osaka = Coordinate::new(35.0, 135.0);
        assert!((baghdad.bearing_to(osaka) - 60.0).abs() < 1.0);
    }

    #[test]
    fn a_fence_reports_crossings_once() {
        let mut fence = Geofence::new(Coordinate::new(37.0, -122.0), 100.0);
        let near = Coordinate::new(37.0005, -122.0); // about 56 m north: inside
        let far = Coordinate::new(37.002, -122.0); // about 222 m north: outside

        assert!(fence.contains(near));
        assert!(!fence.contains(far));

        assert_eq!(fence.update(near), Boundary::Inside);
        assert_eq!(fence.update(far), Boundary::Exited); // the crossing
        assert_eq!(fence.update(far), Boundary::Outside); // still away, no repeat
        assert_eq!(fence.update(near), Boundary::Entered); // back across
        assert_eq!(fence.update(near), Boundary::Inside);
    }

    #[test]
    fn a_point_on_the_boundary_counts_as_inside() {
        // Build a fence whose radius is exactly the distance to a known point.
        let center = Coordinate::new(0.0, 0.0);
        let edge = Coordinate::new(0.0, 1.0);
        let fence = Geofence::new(center, center.distance_to(edge));
        assert!(fence.contains(edge));
    }

    #[test]
    fn a_negative_radius_is_treated_as_its_magnitude() {
        let fence = Geofence::new(Coordinate::new(0.0, 0.0), -100.0);
        assert!(fence.contains(Coordinate::new(0.0, 0.0)));
    }
}
