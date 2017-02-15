#[cfg(feature = "nightly")]
pub mod benches {
    use test::Bencher;

    use slog;
    use slog_term;
    use slog::DrainExt;

    use super::super::*;

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
    fn bench_generate_chunk_geometry(b: &mut Bencher) {
        use render::Vertex;
        use super::super::globe::GlobeGuts;

        let drain = slog_term::streamer().compact().build().fuse();
        let log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));
        let spec = Spec {
            seed: 13,
            floor_radius: 0.91,
            ocean_radius: 1.13,
            block_height: 0.02,
            root_resolution: [32, 64],
            chunk_resolution: [16, 16, 4],
        };
        let globe = Globe::new(spec, &log);
        let spec = globe.spec();
        let globe_view = View::new(spec, &log);
        let mut vertex_data: Vec<Vertex> = Vec::new();
        let mut index_data: Vec<u32> = Vec::new();
        b.iter(|| {
            vertex_data.clear();
            index_data.clear();
            globe_view.make_chunk_geometry(
                &globe,
                globe.chunks()[&globe.chunks().len() / 2].origin,
                &mut vertex_data,
                &mut index_data,
            );
        });
    }
}
