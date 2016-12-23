use super::{ CellPos, Dir };

// TODO: take resolution, too.
pub fn advance(pos: &mut CellPos, dir: &mut Dir) {
    assert_eq!(&Dir::new(0), dir, "Ruh roh, this isn't actually implemented for reals yet.");

    // TODO: actual logic
    pos.x += 1;
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{ CellPos, Dir };

    #[test]
    fn advance_in_positive_x_direction() {
        let mut pos = CellPos::default();
        let mut dir = Dir::default();
        advance(&mut pos, &mut dir);
        assert_eq!(CellPos::default().set_x(1), pos);
        assert_eq!(Dir::default(), dir);
    }
}
