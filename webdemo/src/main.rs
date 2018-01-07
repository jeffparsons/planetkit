#[macro_use]
extern crate stdweb;
extern crate planetkit as pk;

fn main() {
    // Not using this yet in this incarnation of the demo,
    // but I expect to be quite soon. So just leaving it in here for now...
    stdweb::initialize();

    use pk::simple;
    let (mut app, mut window) = simple::new_populated(simple::noop_create_systems);
    app.run(&mut window);
}
