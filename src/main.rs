use nu_plugin::{serve_plugin, JsonSerializer};
use nu_plugin_plot::PluginPlot;

fn main() {
    serve_plugin(&PluginPlot, JsonSerializer {})
}
