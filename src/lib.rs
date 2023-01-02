//! A small crate to plot an ASCII
//! representation of data from nushell

use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, Signature, SyntaxShape, Value};
use textplots::{Chart, Plot, Shape};
pub struct Plotter;

impl Plotter {
    // some functions to do with plot types
    fn plot(&self, call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
        // cli opts
        let max_x_op: Option<u32> = match call.get_flag("max-x") {
            Ok(x) => x.map(|e: i64| e as u32),
            Err(e) => {
                return Err(LabeledError {
                    label: "".into(),
                    msg: format!("Reason: {}", e),
                    span: Some(call.head),
                })
            }
        };

        let max_y_op: Option<u32> = match call.get_flag("max-y") {
            Ok(y) => y.map(|e: i64| e as u32),
            Err(e) => {
                return Err(LabeledError {
                    label: "".into(),
                    msg: format!("Reason: {}", e),
                    span: Some(call.head),
                })
            }
        };

        let max_x = max_x_op.unwrap_or(200);
        let max_y = max_y_op.unwrap_or(50);

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

        let chart = Chart::new(max_x, max_y, min_max_x.0, min_max_x.1)
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
            .named(
                "max-x",
                SyntaxShape::Number,
                "The maximum width of the plot.",
                Some('x'),
            )
            .named(
                "max-y",
                SyntaxShape::Number,
                "The maximum height of the plot.",
                Some('y'),
            )
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
