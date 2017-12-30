extern crate specs;
#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate planetkit as pk;

#[derive(Serialize)]
struct JsMesh {
    pub pos: [f32; 3],
    pub vertexes: Vec<JsVertex>,
    pub indexes: Vec<usize>,
}
js_serializable!(JsMesh);

#[derive(Serialize)]
struct JsVertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
}
js_serializable!(JsVertex);

fn main() {
    stdweb::initialize();

    // Print evidence that we managed to get _something_ to run.
    let globe = pk::globe::Globe::new_earth_scale_example();
    println!("Globe size: {}", globe.spec().floor_radius);

    // Create a world with a dispatcher.
    use pk::simple;
    use pk::types::TimeDeltaResource;
    use pk::globe::Globe;

    println!("Creating the world...");
    let (
        _log,
        mut world,
        dispatcher_builder,
        _movement_input_adapter,
        _mining_input_adapter,
    ) = simple::new_populated_without_window(simple::noop_create_systems);
    let mut dispatcher = dispatcher_builder.build();

    // Tick it along a bit to make sure some chunks get created.
    world.write_resource::<TimeDeltaResource>().0 = 1000.0;

    println!("Dispatching to create chunks...");
    for _ in 0..100 {
        dispatcher.dispatch(&mut world.res);
        world.maintain();
    }

    // Make sure we made some chunks.
    let globes = world.read::<Globe>();
    use specs::Join;
    let globe = globes.join().next().expect("Should have been at least one globe");
    println!("Chunks: {}", globe.chunk_count());

    // See what proto-meshes got created.
    use pk::render::Visual;
    use pk::Spatial;
    let spatials = world.read::<Spatial>();
    let mut visuals = world.write::<Visual>();
    // TODO: clone the logic where we mark them as realized
    // so we can actually just keep ticking and load them as we go.
    for (visual, spatial) in (&mut visuals, &spatials).join() {
        println!("Found a visual!");

        let proto_mesh = match visual.proto_mesh.clone() {
            Some(proto_mesh) => proto_mesh,
            // Never got generated.
            None => continue,
        };

        println!("Found a proto-mesh!");

        let translation_vector = spatial.local_transform().translation.vector;
        let js_mesh = JsMesh {
            vertexes: proto_mesh.vertexes.iter().map(|v|
                JsVertex {
                    pos: [
                        v.a_pos[0],
                        v.a_pos[1],
                        v.a_pos[2],
                    ],
                    color: v.a_color,
                }
            ).collect(),
            indexes: proto_mesh.indexes.iter().map(|i|
                *i as usize
            ).collect(),
            pos: [
                translation_vector.x as f32,
                translation_vector.y as f32,
                translation_vector.z as f32,
            ],
        };

        js! {
            var mesh = @{ js_mesh };
            window.addMesh(mesh);
        };
    }
}
