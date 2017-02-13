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
                &globe.chunks()[&globe.chunks().len() / 2],
                &mut vertex_data,
                &mut index_data,
            );
        });
    }
}
