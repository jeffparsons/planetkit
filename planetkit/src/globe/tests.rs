use super::*;

#[test]
fn find_spawn_points() {
    use rand;

    // Don't bother seeding it off the world spec/seed for now;
    // we just want to check that if we keep finding new points
    // then some of them will fail, because they are underwater.
    let mut rng = rand::thread_rng();
    const TRIALS: usize = 200;
    let successes = (0..TRIALS)
        .map(|_| {
            // TODO: move globe out of closure and "collect garbage" chunks
            // on each iteration? Need to get into the habit of doing this.
            // Maybe also make globe or something record stats on how many
            // chunks loaded over time.
            let mut globe = Globe::new_earth_scale_example();
            globe.air_above_random_surface_dry_land(
                &mut rng, 5, // Min air cells above
                5, // Max distance from starting point
                3, // Max attempts
            )
        })
        .filter(|maybe_pos| maybe_pos.is_some())
        .count();
    // Some should eventually succeed, some should give up.
    assert!(successes > 5);
    assert!(successes < TRIALS - 5);
}

#[cfg(feature = "nightly")]
pub mod benches {
    use test::Bencher;

    use slog;

    use super::super::*;

    use globe::GridPoint3;

    #[bench]
    // # History for picking the "middle of the vector" chunk.
    //
    // - Original `cull_more_faces_impractically_slow` culling implementation:
    //     - 3,727,934 ns/iter (+/- 391,582
    // - After introducing `Cursor`:
    //     - 3,618,305 ns/iter (+/- 539,063)
    //     - No noticeable change; build_chunk_geometry already only operates
    //       directly on a single chunk. It's the implementation of `cull_cell`,
    //       and the underlying implementation of `Neighbors` that make it so
    //       horrendously slow at the moment.
    // - After using `Cursor` in `cull_cell`:
    //     - 861,702 ns/iter (+/- 170,677)
    //     - Substantially better, but there are many more gains to be had.
    // - After cleaning up implementation and use of `Neighbors`:
    //     - 565,896 ns/iter (+/- 237,193
    //     - A little bit better, but mostly by eliminating completely useless
    //       checks for diagonal neighbors. The next wins will come from implementing
    //       an "easy case" version of `Neighbors` that avoids most of the math.
    // - After implementing fast `Neighbors`:
    //     - 486,945 ns/iter (+/- 112,408)
    //     - Only a tiny speed-up. There's lots more room for improvement on this front,
    //       but given that my chunks at the moment are very small, I'm just going to
    //       leave it as is and move on to bigger fish.
    // - After replacing chunk vector with hash map:
    //     - 426,929 ns/iter (+/- 56,074)
    //     - Again, only a very small improvement. This change wasn't made to speed
    //       up generating chunk geometry; I'm just updating this history for completeness.
    //     - I believe this change would have been a lot more noticeable if I hadn't
    //       already implemented `Cursor`, because it speeds up looking up a chunk
    //       by its origin, which `Cursor` helps you avoid most of the time.
    fn bench_generate_chunk_geometry(b: &mut Bencher) {
        use render::Vertex;

        const ROOT_RESOLUTION: [GridCoord; 2] = [32, 64];
        const CHUNK_RESOLUTION: [GridCoord; 3] = [16, 16, 4];

        let drain = slog::Discard;
        let log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));
        let spec = Spec::new(13, 0.91, 1.13, 0.02, ROOT_RESOLUTION, CHUNK_RESOLUTION);
        let globe = Globe::new(spec);
        let spec = globe.spec();
        let globe_view = View::new(spec, &log);
        let mut vertex_data: Vec<Vertex> = Vec::new();
        let mut index_data: Vec<u32> = Vec::new();
        // Copied from output of old version of test to make sure
        // we're actually benchmarking the same thing.
        let middle_chunk_origin = ChunkOrigin::new(
            GridPoint3::new(2.into(), 16, 16, 8),
            ROOT_RESOLUTION,
            CHUNK_RESOLUTION,
        );
        b.iter(|| {
            vertex_data.clear();
            index_data.clear();
            globe_view.make_chunk_geometry(
                &globe,
                middle_chunk_origin,
                &mut vertex_data,
                &mut index_data,
            );
        });
    }
}
