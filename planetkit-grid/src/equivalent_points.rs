use std::slice;

use arrayvec;

use super::ROOTS;
use super::{GridCoord, Point2, Point3, Root};

// We need to handle 9 different cases:
//
//            1
//      ◌     ●     ◌
//     / \  4/ \3  / \
//    /   \ /   \ /   \
//   ◌     ●     ●     ◌
//    \     \  9  \5    \
//     \    6\     \     \
//      ◌     ●     ●     ◌
//       \   / \8 7/ \   /
//        \ /   \ /   \ /
//         ◌     ●     ◌
//               2
//
enum EquivalentPointsImpl {
    NorthPole(NorthPolePoints),
    SouthPole(SouthPolePoints),
    EastArctic(EastArcticPoints),
    WestArctic(WestArcticPoints),
    EastTropics(EastTropicsPoints),
    WestTropics(WestTropicsPoints),
    EastAntarctic(EastAntarcticPoints),
    WestAntarctic(WestAntarcticPoints),
    Interior(InteriorPoints),
}

/// Iterator over points in all roots that are equivalent to the given point.
///
/// This includes the given point, not just the _other_ points that are equivalent to it.
/// Therefore for most points (specifically those not on the edge of a root) this will yield a single
/// item that is that same given point.
///
/// The order of points yielded is arbitrary.
pub struct EquivalentPoints {
    iter: EquivalentPointsImpl,
}

impl EquivalentPoints {
    pub fn new(point: Point3, root_resolution: [GridCoord; 2]) -> EquivalentPoints {
        if point.x == 0 && point.y == 0 {
            EquivalentPoints {
                iter: EquivalentPointsImpl::NorthPole(NorthPolePoints::new(point)),
            }
        } else if point.x == root_resolution[0] && point.y == root_resolution[1] {
            EquivalentPoints {
                iter: EquivalentPointsImpl::SouthPole(SouthPolePoints::new(point, root_resolution)),
            }
        } else if point.x == 0 && point.y < root_resolution[0] {
            // Above, remember that x-resolution is always half y-resolution.
            EquivalentPoints {
                iter: EquivalentPointsImpl::EastArctic(EastArcticPoints::new(point)),
            }
        } else if point.y == 0 {
            EquivalentPoints {
                iter: EquivalentPointsImpl::WestArctic(WestArcticPoints::new(point)),
            }
        } else if point.x == 0 && point.y >= root_resolution[0] {
            EquivalentPoints {
                iter: EquivalentPointsImpl::EastTropics(EastTropicsPoints::new(
                    point,
                    root_resolution,
                )),
            }
        } else if point.x == root_resolution[0] && point.y < root_resolution[0] {
            EquivalentPoints {
                iter: EquivalentPointsImpl::WestTropics(WestTropicsPoints::new(
                    point,
                    root_resolution,
                )),
            }
        } else if point.y == root_resolution[1] {
            EquivalentPoints {
                iter: EquivalentPointsImpl::EastAntarctic(EastAntarcticPoints::new(
                    point,
                    root_resolution,
                )),
            }
        } else if point.x == root_resolution[0] && point.y >= root_resolution[0] {
            EquivalentPoints {
                iter: EquivalentPointsImpl::WestAntarctic(WestAntarcticPoints::new(
                    point,
                    root_resolution,
                )),
            }
        } else {
            EquivalentPoints {
                iter: EquivalentPointsImpl::Interior(InteriorPoints::new(point)),
            }
        }
    }
}

impl Iterator for EquivalentPoints {
    type Item = Point3;

    fn next(&mut self) -> Option<Point3> {
        match self.iter {
            EquivalentPointsImpl::NorthPole(ref mut iter) => iter.next(),
            EquivalentPointsImpl::SouthPole(ref mut iter) => iter.next(),
            EquivalentPointsImpl::EastArctic(ref mut iter) => iter.next(),
            EquivalentPointsImpl::WestArctic(ref mut iter) => iter.next(),
            EquivalentPointsImpl::EastTropics(ref mut iter) => iter.next(),
            EquivalentPointsImpl::WestTropics(ref mut iter) => iter.next(),
            EquivalentPointsImpl::EastAntarctic(ref mut iter) => iter.next(),
            EquivalentPointsImpl::WestAntarctic(ref mut iter) => iter.next(),
            EquivalentPointsImpl::Interior(ref mut iter) => iter.next(),
        }
    }
}

//
// 1. North pole
//

struct NorthPolePoints {
    z: GridCoord,
    roots_iter: slice::Iter<'static, Root>,
}

impl NorthPolePoints {
    fn new(point: Point3) -> NorthPolePoints {
        NorthPolePoints {
            z: point.z,
            roots_iter: ROOTS.iter(),
        }
    }
}

impl Iterator for NorthPolePoints {
    type Item = Point3;

    fn next(&mut self) -> Option<Point3> {
        self.roots_iter.next().map(|root| Point3 {
            rxy: Point2 {
                root: *root,
                x: 0,
                y: 0,
            },
            z: self.z,
        })
    }
}

//
// 2. South pole
//

struct SouthPolePoints {
    x: GridCoord,
    y: GridCoord,
    z: GridCoord,
    roots_iter: slice::Iter<'static, Root>,
}

impl SouthPolePoints {
    fn new(point: Point3, root_resolution: [GridCoord; 2]) -> SouthPolePoints {
        SouthPolePoints {
            x: root_resolution[0],
            y: root_resolution[1],
            z: point.z,
            roots_iter: ROOTS.iter(),
        }
    }
}

impl Iterator for SouthPolePoints {
    type Item = Point3;

    fn next(&mut self) -> Option<Point3> {
        self.roots_iter.next().map(|root| Point3 {
            rxy: Point2 {
                root: *root,
                x: self.x,
                y: self.y,
            },
            z: self.z,
        })
    }
}

//
// 3. East arctic
//

struct EastArcticPoints {
    points_iter: arrayvec::IntoIter<[Point3; 2]>,
}

impl EastArcticPoints {
    fn new(point: Point3) -> EastArcticPoints {
        use arrayvec::ArrayVec;
        let mut points: ArrayVec<[Point3; 2]> = ArrayVec::new();
        points.push(point);
        points.push(Point3::new(
            point.root.next_east(),
            // y-axis in arctic maps to x-axis in next root east.
            point.y,
            0,
            point.z,
        ));
        EastArcticPoints {
            points_iter: points.into_iter(),
        }
    }
}

impl Iterator for EastArcticPoints {
    type Item = Point3;

    fn next(&mut self) -> Option<Point3> {
        self.points_iter.next()
    }
}

//
// 4. West arctic
//

struct WestArcticPoints {
    points_iter: arrayvec::IntoIter<[Point3; 2]>,
}

impl WestArcticPoints {
    fn new(point: Point3) -> WestArcticPoints {
        use arrayvec::ArrayVec;
        let mut points: ArrayVec<[Point3; 2]> = ArrayVec::new();
        points.push(point);
        points.push(Point3::new(
            point.root.next_west(),
            // x-axis in arctic maps to y-axis in next root west.
            0,
            point.x,
            point.z,
        ));
        WestArcticPoints {
            points_iter: points.into_iter(),
        }
    }
}

impl Iterator for WestArcticPoints {
    type Item = Point3;

    fn next(&mut self) -> Option<Point3> {
        self.points_iter.next()
    }
}

//
// 5. East tropics
//

struct EastTropicsPoints {
    points_iter: arrayvec::IntoIter<[Point3; 2]>,
}

impl EastTropicsPoints {
    fn new(point: Point3, root_resolution: [GridCoord; 2]) -> EastTropicsPoints {
        use arrayvec::ArrayVec;
        let mut points: ArrayVec<[Point3; 2]> = ArrayVec::new();
        points.push(point);
        points.push(Point3::new(
            point.root.next_east(),
            // y-axis in tropics maps to y-axis in next root east,
            // but offset and with max-x.
            root_resolution[0],
            point.y - root_resolution[0],
            point.z,
        ));
        EastTropicsPoints {
            points_iter: points.into_iter(),
        }
    }
}

impl Iterator for EastTropicsPoints {
    type Item = Point3;

    fn next(&mut self) -> Option<Point3> {
        self.points_iter.next()
    }
}

//
// 6. West tropics
//

struct WestTropicsPoints {
    points_iter: arrayvec::IntoIter<[Point3; 2]>,
}

impl WestTropicsPoints {
    fn new(point: Point3, root_resolution: [GridCoord; 2]) -> WestTropicsPoints {
        use arrayvec::ArrayVec;
        let mut points: ArrayVec<[Point3; 2]> = ArrayVec::new();
        points.push(point);
        points.push(Point3::new(
            point.root.next_west(),
            // y-axis at max-x in tropics maps to y-axis in next root east,
            // but offset and with min-x.
            0,
            point.y + root_resolution[0],
            point.z,
        ));
        WestTropicsPoints {
            points_iter: points.into_iter(),
        }
    }
}

impl Iterator for WestTropicsPoints {
    type Item = Point3;

    fn next(&mut self) -> Option<Point3> {
        self.points_iter.next()
    }
}

//
// 7. East antarctic
//

struct EastAntarcticPoints {
    points_iter: arrayvec::IntoIter<[Point3; 2]>,
}

impl EastAntarcticPoints {
    fn new(point: Point3, root_resolution: [GridCoord; 2]) -> EastAntarcticPoints {
        use arrayvec::ArrayVec;
        let mut points: ArrayVec<[Point3; 2]> = ArrayVec::new();
        points.push(point);
        points.push(Point3::new(
            point.root.next_east(),
            // x-axis at max-y in antarctic maps to y-axis in next root east,
            // but offset and with max-x.
            root_resolution[0],
            point.x + root_resolution[0],
            point.z,
        ));
        EastAntarcticPoints {
            points_iter: points.into_iter(),
        }
    }
}

impl Iterator for EastAntarcticPoints {
    type Item = Point3;

    fn next(&mut self) -> Option<Point3> {
        self.points_iter.next()
    }
}

//
// 8. West antarctic
//

struct WestAntarcticPoints {
    points_iter: arrayvec::IntoIter<[Point3; 2]>,
}

impl WestAntarcticPoints {
    fn new(point: Point3, root_resolution: [GridCoord; 2]) -> WestAntarcticPoints {
        use arrayvec::ArrayVec;
        let mut points: ArrayVec<[Point3; 2]> = ArrayVec::new();
        points.push(point);
        points.push(Point3::new(
            point.root.next_west(),
            // y-axis in antarctic maps to x-axis in next root east,
            // but offset and with max-y.
            point.y - root_resolution[0],
            root_resolution[1],
            point.z,
        ));
        WestAntarcticPoints {
            points_iter: points.into_iter(),
        }
    }
}

impl Iterator for WestAntarcticPoints {
    type Item = Point3;

    fn next(&mut self) -> Option<Point3> {
        self.points_iter.next()
    }
}

//
// 9. Interior / not on any root quad edge
//

struct InteriorPoints {
    points_iter: arrayvec::IntoIter<[Point3; 1]>,
}

impl InteriorPoints {
    fn new(point: Point3) -> InteriorPoints {
        use arrayvec::ArrayVec;
        let mut points: ArrayVec<[Point3; 1]> = ArrayVec::new();
        points.push(point);
        InteriorPoints {
            points_iter: points.into_iter(),
        }
    }
}

impl Iterator for InteriorPoints {
    type Item = Point3;

    fn next(&mut self) -> Option<Point3> {
        self.points_iter.next()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::semi_arbitrary_compare;

    #[test]
    fn points_equivalent_to_north_pole() {
        const ROOT_RESOLUTION: [GridCoord; 2] = [8, 16];
        let point = Point3::new(
            // Arbitrary root
            4.into(),
            // North pole
            0,
            0,
            // Arbitrary z-coordinate to check below
            77,
        );
        let points_iter = EquivalentPoints::new(point, ROOT_RESOLUTION);
        let mut equivalent_points: Vec<Point3> = points_iter.collect();
        equivalent_points.sort_by(semi_arbitrary_compare);
        assert_eq!(equivalent_points.len(), 5);
        assert!(
            equivalent_points
                == vec![
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 0 },
                            x: 0,
                            y: 0,
                        },
                        z: 77,
                    },
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 1 },
                            x: 0,
                            y: 0,
                        },
                        z: 77,
                    },
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 2 },
                            x: 0,
                            y: 0,
                        },
                        z: 77,
                    },
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 3 },
                            x: 0,
                            y: 0,
                        },
                        z: 77,
                    },
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 4 },
                            x: 0,
                            y: 0,
                        },
                        z: 77,
                    },
                ]
        );
    }

    #[test]
    fn points_equivalent_to_south_pole() {
        const ROOT_RESOLUTION: [GridCoord; 2] = [8, 16];
        let point = Point3::new(
            // Arbitrary root
            4.into(),
            // South pole
            ROOT_RESOLUTION[0],
            ROOT_RESOLUTION[1],
            // Arbitrary z-coordinate to check below
            77,
        );
        let points_iter = EquivalentPoints::new(point, ROOT_RESOLUTION);
        let mut equivalent_points: Vec<Point3> = points_iter.collect();
        equivalent_points.sort_by(semi_arbitrary_compare);
        assert_eq!(equivalent_points.len(), 5);
        assert!(
            equivalent_points
                == vec![
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 0 },
                            x: ROOT_RESOLUTION[0],
                            y: ROOT_RESOLUTION[1],
                        },
                        z: 77,
                    },
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 1 },
                            x: ROOT_RESOLUTION[0],
                            y: ROOT_RESOLUTION[1],
                        },
                        z: 77,
                    },
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 2 },
                            x: ROOT_RESOLUTION[0],
                            y: ROOT_RESOLUTION[1],
                        },
                        z: 77,
                    },
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 3 },
                            x: ROOT_RESOLUTION[0],
                            y: ROOT_RESOLUTION[1],
                        },
                        z: 77,
                    },
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 4 },
                            x: ROOT_RESOLUTION[0],
                            y: ROOT_RESOLUTION[1],
                        },
                        z: 77,
                    },
                ]
        );
    }

    #[test]
    fn points_equivalent_to_east_arctic() {
        const ROOT_RESOLUTION: [GridCoord; 2] = [8, 16];
        let point = Point3::new(
            // Arbitrary root
            3.into(),
            // East arctic
            0,
            3,
            // Arbitrary z-coordinate to check below
            77,
        );
        let points_iter = EquivalentPoints::new(point, ROOT_RESOLUTION);
        let mut equivalent_points: Vec<Point3> = points_iter.collect();
        equivalent_points.sort_by(semi_arbitrary_compare);
        assert_eq!(equivalent_points.len(), 2);
        assert!(
            equivalent_points
                == vec![
                    // Same point as given
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 3 },
                            x: 0,
                            y: 3,
                        },
                        z: 77,
                    },
                    // Equivalent point in next root east
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 4 },
                            x: 3,
                            y: 0,
                        },
                        z: 77,
                    },
                ]
        );
    }

    #[test]
    fn points_equivalent_to_west_arctic() {
        const ROOT_RESOLUTION: [GridCoord; 2] = [8, 16];
        let point = Point3::new(
            // Arbitrary root
            3.into(),
            // West arctic
            3,
            0,
            // Arbitrary z-coordinate to check below
            77,
        );
        let points_iter = EquivalentPoints::new(point, ROOT_RESOLUTION);
        let mut equivalent_points: Vec<Point3> = points_iter.collect();
        equivalent_points.sort_by(semi_arbitrary_compare);
        assert_eq!(equivalent_points.len(), 2);
        assert!(
            equivalent_points
                == vec![
                    // Equivalent point in next root east
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 2 },
                            x: 0,
                            y: 3,
                        },
                        z: 77,
                    },
                    // Same point as given
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 3 },
                            x: 3,
                            y: 0,
                        },
                        z: 77,
                    },
                ]
        );
    }

    #[test]
    fn points_equivalent_to_east_tropics() {
        const ROOT_RESOLUTION: [GridCoord; 2] = [8, 16];
        let point = Point3::new(
            // Arbitrary root
            3.into(),
            // East tropics
            0,
            13,
            // Arbitrary z-coordinate to check below
            77,
        );
        let points_iter = EquivalentPoints::new(point, ROOT_RESOLUTION);
        let mut equivalent_points: Vec<Point3> = points_iter.collect();
        equivalent_points.sort_by(semi_arbitrary_compare);
        assert_eq!(equivalent_points.len(), 2);
        assert!(
            equivalent_points
                == vec![
                    // Same point as given
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 3 },
                            x: 0,
                            y: 13,
                        },
                        z: 77,
                    },
                    // Equivalent point in next root east
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 4 },
                            x: 8,
                            y: 5,
                        },
                        z: 77,
                    },
                ]
        );
    }

    #[test]
    fn points_equivalent_to_west_tropics() {
        const ROOT_RESOLUTION: [GridCoord; 2] = [8, 16];
        let point = Point3::new(
            // Arbitrary root
            3.into(),
            // West tropics
            8,
            4,
            // Arbitrary z-coordinate to check below
            77,
        );
        let points_iter = EquivalentPoints::new(point, ROOT_RESOLUTION);
        let mut equivalent_points: Vec<Point3> = points_iter.collect();
        equivalent_points.sort_by(semi_arbitrary_compare);
        assert_eq!(equivalent_points.len(), 2);
        assert!(
            equivalent_points
                == vec![
                    // Equivalent point in next root east
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 2 },
                            x: 0,
                            y: 12,
                        },
                        z: 77,
                    },
                    // Same point as given
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 3 },
                            x: 8,
                            y: 4,
                        },
                        z: 77,
                    },
                ]
        );
    }

    #[test]
    fn points_equivalent_to_east_antarctic() {
        const ROOT_RESOLUTION: [GridCoord; 2] = [8, 16];
        let point = Point3::new(
            // Arbitrary root
            3.into(),
            // East arctic
            3,
            16,
            // Arbitrary z-coordinate to check below
            77,
        );
        let points_iter = EquivalentPoints::new(point, ROOT_RESOLUTION);
        let mut equivalent_points: Vec<Point3> = points_iter.collect();
        equivalent_points.sort_by(semi_arbitrary_compare);
        assert_eq!(equivalent_points.len(), 2);
        assert!(
            equivalent_points
                == vec![
                    // Same point as given
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 3 },
                            x: 3,
                            y: 16,
                        },
                        z: 77,
                    },
                    // Equivalent point in next root east
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 4 },
                            x: 8,
                            y: 11,
                        },
                        z: 77,
                    },
                ]
        );
    }

    #[test]
    fn points_equivalent_to_west_antarctic() {
        const ROOT_RESOLUTION: [GridCoord; 2] = [8, 16];
        let point = Point3::new(
            // Arbitrary root
            4.into(),
            // East arctic
            8,
            11,
            // Arbitrary z-coordinate to check below
            77,
        );
        let points_iter = EquivalentPoints::new(point, ROOT_RESOLUTION);
        let mut equivalent_points: Vec<Point3> = points_iter.collect();
        equivalent_points.sort_by(semi_arbitrary_compare);
        assert_eq!(equivalent_points.len(), 2);
        assert!(
            equivalent_points
                == vec![
                    // Equivalent point in next root west
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 3 },
                            x: 3,
                            y: 16,
                        },
                        z: 77,
                    },
                    // Same point as given
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 4 },
                            x: 8,
                            y: 11,
                        },
                        z: 77,
                    },
                ]
        );
    }

    #[test]
    fn points_equivalent_to_interior() {
        const ROOT_RESOLUTION: [GridCoord; 2] = [8, 16];
        let point = Point3::new(
            // Arbitrary root
            4.into(),
            // Not on any root boundary
            3,
            5,
            // Arbitrary z-coordinate to check below
            77,
        );
        let points_iter = EquivalentPoints::new(point, ROOT_RESOLUTION);
        let mut equivalent_points: Vec<Point3> = points_iter.collect();
        equivalent_points.sort_by(semi_arbitrary_compare);
        assert_eq!(equivalent_points.len(), 1);
        assert!(
            equivalent_points
                == vec![
                    // Same point as given
                    Point3 {
                        rxy: Point2 {
                            root: Root { index: 4 },
                            x: 3,
                            y: 5,
                        },
                        z: 77,
                    },
                ]
        );
    }

    #[test]
    fn all_equivalent_points_symmetric() {
        const ROOT_RESOLUTION: [GridCoord; 2] = [16, 32];
        // NOTE: Going right up to the far edge.
        for xy in iproduct!(0..=ROOT_RESOLUTION[0], 0..=ROOT_RESOLUTION[1]) {
            let (x, y) = xy;
            let point = Point3::new(
                // Arbitrary root
                4.into(),
                x,
                y,
                // Arbitrary z-coordinate
                77,
            );
            let points_iter = EquivalentPoints::new(point, ROOT_RESOLUTION);
            let equivalent_points: HashSet<Point3> = points_iter.collect();
            // Make sure that the set of points equivalent to _this_ point is the same
            // as the set we just found.
            for point2 in &equivalent_points {
                let points_iter2 = EquivalentPoints::new(*point2, ROOT_RESOLUTION);
                let equivalent_points2: HashSet<Point3> = points_iter2.collect();
                assert_eq!(equivalent_points, equivalent_points2);
            }
        }
    }
}
