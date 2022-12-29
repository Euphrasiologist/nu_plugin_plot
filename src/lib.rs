//! A small crate to plot an ASCII
//! representation of data from nushell

use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, Signature, Value};

pub struct Plot;

impl Plot {
    // some functions to do with plot types
    fn plot(call: &EvaluatedCall, _input: &Value) -> Result<Value, LabeledError> {
        eprintln!("Hello world!");
        Ok(Value::Nothing { span: call.head })
    }
}

impl Plugin for Plot {
    // Try and keep it one command with a few flags
    fn signature(&self) -> Vec<Signature> {
        vec![Signature::build("plot")
            .usage("Render an ASCII plot from a list of values.")
            .category(Category::Experimental)]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        match name {
            "plot" => Plot::plot(call, input),
            _ => Err(LabeledError {
                label: "Plugin call with wrong name signature".into(),
                msg: "the signature used to call the plugin does not match any name in the plugin signature vector".into(),
                span: Some(call.head),
            }),
        }
    }
}
