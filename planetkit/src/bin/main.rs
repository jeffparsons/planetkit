extern crate planetkit as pk;

fn main() {
    let (mut app, mut window) = pk::simple::new_populated();
    app.run(&mut window);
}
