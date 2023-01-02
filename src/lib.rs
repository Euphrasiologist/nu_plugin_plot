//! A small crate to plot an ASCII
//! representation of data from nushell

use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, Signature, Value};
use textplots::{Chart, Plot, Shape};
pub struct Plotter;

impl Plotter {
    // some functions to do with plot types
    fn plot(&self, call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
        // cli opts
        // let max_x = call.get_flag("max-x")?;
        let values = match input.as_list() {
            Ok(v) => v,
            Err(e) => panic!("{:?}", e),
        };

        let v: Vec<(f32, f32)> = values
            .iter()
            .enumerate()
            .map(|(i, e)| (i as f32, e.as_f64().unwrap() as f32))
            .collect();

        // min/max
        fn min_max(series: &[f32]) -> (f32, f32) {
            let min = series
                .iter()
                .fold(std::f32::MAX, |accu, &x| if x < accu { x } else { accu });
            let max = series
                .iter()
                .fold(std::f32::MIN, |accu, &x| if x > accu { x } else { accu });
            (min, max)
        }

        let min_max_x = {
            let x: Vec<f32> = v.iter().map(|e| e.0).collect();
            min_max(&x)
        };

        let chart = Chart::new(250, 50, min_max_x.0, min_max_x.1)
            .lineplot(&Shape::Lines(&v))
            .to_string();

        Ok(Value::String {
            val: chart,
            span: call.head,
        })
    }
}

impl Plugin for Plotter {
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
            "plot" => self.plot(call, input),
            _ => Err(LabeledError {
                label: "Plugin call with wrong name signature".into(),
                msg: "the signature used to call the plugin does not match any name in the plugin signature vector".into(),
                span: Some(call.head),
            }),
        }
    }
}
