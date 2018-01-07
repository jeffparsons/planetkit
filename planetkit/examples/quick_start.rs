extern crate planetkit as pk;

fn main() {
    let mut app = pk::AppBuilder::new()
        .add_common_systems()
        .build_gui();
    pk::simple::populate_world(app.world_mut());
    app.run();
}
