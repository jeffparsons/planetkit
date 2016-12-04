use na::{ Point2, Point3, Vector2, Vector3 };

// Common types for all of PlanetKit.
//
// REVISIT: should some of these actually be `f32`
// for performance reasons? We definitely want
// `f64` for doing the non-realtime geometric
// manipulations, but entity positions etc. don't
// really need it.
pub type Vec2 = Vector2<f64>;
pub type Vec3 = Vector3<f64>;
pub type Pt2 = Point2<f64>;
pub type Pt3 = Point3<f64>;

pub type TimeDelta = f64;
