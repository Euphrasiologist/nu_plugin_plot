//! A small crate to plot an ASCII
//! representation of data from nushell

use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, Signature, SyntaxShape, Type, Value};
use rgb::RGB8;
use textplots::{Chart, Plot, Shape};
pub struct Plotter;

/// So the chart is not hard up against the left of the terminal.
const TAB: &str = "    ";

/// Colors, five of them.
const COLORS: &[RGB8] = &[
    // dark turquoise
    RGB8 {
        r: 0,
        g: 206,
        b: 209,
    },
    // orange
    RGB8 {
        r: 255,
        g: 165,
        b: 0,
    },
    // lime
    RGB8 { r: 0, g: 255, b: 0 },
    // blue
    RGB8 { r: 0, g: 0, b: 255 },
    // pink
    RGB8 {
        r: 255,
        g: 20,
        b: 147,
    },
];

impl Plotter {
    // some functions to do with plot types
    fn plot(&self, call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
        // cli opts
        let max_x_op: Option<u32> = call.get_flag("max-x").map(|e| e.map(|f: i64| f as u32))?;
        let max_y_op: Option<u32> = call.get_flag("max-y").map(|e| e.map(|f: i64| f as u32))?;

        let max_x = max_x_op.unwrap_or(200);
        let max_y = max_y_op.unwrap_or(50);

        let values = input.as_list()?;

        let v: Result<Vec<(f32, f32)>, LabeledError> = values
            .iter()
            .enumerate()
            .map(|(i, e)| match e {
                Value::Int { val: _, span: _ } => Ok((i as f32, e.as_integer()? as f32)),
                Value::Float { val: _, span: _ } => Ok((i as f32, e.as_f64()? as f32)),
                e => Err(LabeledError {
                    label: "Incorrect type supplied.".into(),
                    msg: format!("Got {}, need integer or float.", e.get_type()),
                    span: Some(call.head),
                }),
            })
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
            let x: Vec<f32> = v.clone().unwrap().iter().map(|e| e.0).collect();
            min_max(&x)
        };

        let chart = Chart::new(max_x, max_y, min_max_x.0, min_max_x.1)
            .lineplot(&Shape::Lines(&v.unwrap()))
            .to_string();

        let chart = TAB.to_owned() + &chart.replace('\n', &format!("\n{}", TAB));

        Ok(Value::String {
            val: chart,
            span: call.head,
        })
    }
}

/// Get the type of a value, and its length if it's a list.
fn get_value_type_or_list_length(val: &Value) -> (Type, Option<usize>) {
    let typ = val.get_type();
    let len = match val.as_list() {
        Ok(l) => Some(l.len()),
        Err(_) => None,
    };

    (typ, len)
}

/// Check a list of values for equality of type,
/// length. Return the type.
fn check_equality_of_list(
    l: &[Value],
    call: &EvaluatedCall,
) -> Result<(Type, Option<usize>), LabeledError> {
    let mut types = vec![];
    let mut len_ops = vec![];

    for val in l {
        let (typ, len_op) = get_value_type_or_list_length(val);
        types.push(typ);
        len_ops.push(len_op);
    }

    // check types are all the same
    // e.g. Int/Float/List
    let first_type = &types[0];
    let check_type_pass = types.iter().all(|e| e == first_type);

    if !check_type_pass {
        return Err(LabeledError {
            label: "Type differences.".into(),
            msg: "Can't plot a list of multiple types.".into(),
            span: Some(call.head),
        });
    }

    let first_len_op = &len_ops[0];
    let check_len_pass = len_ops.iter().all(|e| e == first_len_op);

    if !check_len_pass {
        return Err(LabeledError {
            label: "List length differences.".into(),
            msg: "Can't plot a list of differing length lists.".into(),
            span: Some(call.head),
        });
    }

    Ok((first_type.clone(), *first_len_op))
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
            "plot" => {
                // here we want to check what the input is.
                match input.as_list() {
                    Ok(list) => {
                        // so we have a list. what's in it? we need to check each inner value
                        if list.is_empty() {
                            return Err(LabeledError { label: "No elements in the list.".into(), msg: "Can't plot a zero element list.".into(), span: Some(call.head) })
                        }

                        let (value_type, list_len_op) = check_equality_of_list(list, call)?;

                        // if in fact we have a nested list
                        if let Some(len) = list_len_op {
                            // we haven't implemented this yet
                            Ok(Value::String { val: format!("This nested list has {} elements in each.", len), span: call.head })
                        } else {
                            // we have a normal plot, single list of numbers
                            match value_type {
                                Type::Float => self.plot(call, input),
                                Type::Int => self.plot(call, input),
                                e =>  Err(LabeledError { label: "Incorrect List type.".into(), msg: format!("List type is {}, but should be float or int.", e), span: Some(call.head) })
                            }
                        }
                    },
                    Err(e) => return Err(LabeledError { label: "Incorrect input type.".into(), msg: format!("Input type is {}, but should be a List.", e), span: Some(call.head) }),
                }
            }
            _ => Err(LabeledError {
                label: "Plugin call with wrong name signature".into(),
                msg: "the signature used to call the plugin does not match any name in the plugin signature vector".into(),
                span: Some(call.head),
            }),
        }
    }
}
