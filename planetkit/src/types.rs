use crate::na;

// TODO: Derive everything else from this.
// TODO: Rename; this is too likely to end up in too
// many scopes to have such a general name.
// Maybe "PKFloat"?
//
// _OR_ just really discourage importing everything
// from here, and encourage using as, e.g., `pk::Real`,
// `pk::Point3`. That's probably better. Then you can also
// encourage referring to the _grid_ types as, e.g.,
// `gg::Point3` or `grid::Point3` to avoid ambiguity.
pub type Real = f64;

// Common types for all of PlanetKit.
//
// REVISIT: should some of these actually be `f32`
// for performance reasons? We definitely want
// `f64` for doing the non-realtime geometric
// manipulations, but entity positions etc. don't
// really need it.
pub type Vec2 = na::Vector2<f64>;
pub type Vec3 = na::Vector3<f64>;
pub type Pt2 = na::Point2<f64>;
pub type Pt3 = na::Point3<f64>;
pub type Rot3 = na::Rotation3<f64>;
pub type Iso3 = na::Isometry3<f64>;

pub type TimeDelta = f64;

#[derive(Default)]
pub struct TimeDeltaResource(pub TimeDelta);

pub type Mat4 = na::Matrix4<f64>;
