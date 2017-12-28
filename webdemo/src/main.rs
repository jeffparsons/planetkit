extern crate planetkit as pk;

fn main() {
    // Print evidence that we managed to get _something_ to run.
    let globe = pk::globe::Globe::new_earth_scale_example();
    println!("Globe size: {}", globe.spec().floor_radius);

    // TODO: Separate driving the game logic from 'App',
    // so we can create a simple world here and start ticking
    // it along.
}
