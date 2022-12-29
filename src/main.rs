use nu_plugin::{serve_plugin, JsonSerializer};
use nu_plugin_plot::Plot;

fn main() {
    serve_plugin(&mut Plot {}, JsonSerializer {})
}