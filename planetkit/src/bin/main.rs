extern crate planetkit as pk;

fn main() {
    use pk::simple;

    let (mut app, mut window) = simple::new_populated(simple::noop_create_systems);
    app.run(&mut window);
}
