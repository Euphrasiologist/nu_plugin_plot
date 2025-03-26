use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_plot::PluginPlot;

fn main() {
    serve_plugin(&PluginPlot {}, MsgPackSerializer {})
}
