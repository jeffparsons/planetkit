extern crate planetkit as pk;

fn main() {
    let (mut app, mut window) = pk::simple::new();
    app.run(&mut window);
}
