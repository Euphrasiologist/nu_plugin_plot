use nu_plugin::{serve_plugin, JsonSerializer};
use nu_plugin_plot::Plotter;

fn main() {
    serve_plugin(&mut Plotter {}, JsonSerializer {})
}
