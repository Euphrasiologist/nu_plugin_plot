//! A small crate to plot an ASCII
//! representation of a List data type from nushell
//!
//! Three commands are supplied.
//! - `plot` plots a 1-dimensional numeric list/nested list
//! - `hist` plots a 1-dimensional numeric list/nested list
//! - `xyplot` plots a 2-dimensional numeric list (nested list with length == 2)

use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, PluginSignature, SyntaxShape, Type, Value};
pub mod color_plot;

use color_plot::drawille::PixelColor;
use color_plot::textplots::{utils::histogram, Chart, ColorPlot, Plot, Shape};
use owo_colors::OwoColorize;

/// The type of plot to make. Probably don't need this
/// enum, but made me think more clearly.
enum PlotType {
    /// Normal indexed numeric plot.
    Plot,
    /// Internally compute the histogram of a list of
    /// numbers and then plot.
    Hist,
    /// A bivariate plot, a nested list of length 2.
    XYplot,
}

impl PlotType {
    fn from(string: &str) -> Self {
        match string {
            "plot" => Self::Plot,
            "hist" => Self::Hist,
            "xyplot" => Self::XYplot,
            _ => unimplemented!(),
        }
    }
}

/// `Plotter` struct passed to nu.
pub struct Plotter;

/// So the chart is not hard up against the left of the terminal.
const TAB: &str = "    ";

/// Colors, five of them.
const COLORS: &[PixelColor] = &[
    PixelColor::BrightWhite,
    PixelColor::BrightRed,
    PixelColor::BrightBlue,
    PixelColor::BrightYellow,
    PixelColor::Cyan,
];

/// The command line options.
///
/// These apply to `plot`, `hist`, and `xyplot`.
struct CliOpts {
    /// The maximum y height of the plot.
    height_op: Option<u32>,
    /// The maximum x width of the plot.
    width_op: Option<u32>,
    /// Add a legend to the plot.
    legend: bool,
    /// Render a step plot, instead of a line plot.
    steps: bool,
    /// Render a bar plot, instead of a line plot.
    bars: bool,
    /// Render single points, instead of line plot.
    points: bool,
    /// Add a title to the plot.
    title: Option<String>,
    /// Number of bins in the histogram
    bins: Option<u32>,
}

/// Parse the command line options.
fn parse_cli_opts(call: &EvaluatedCall) -> Result<CliOpts, LabeledError> {
    // scale the width and height to the size of the terminal unless specified on the cli
    let height_op: Option<u32> = call.get_flag("height").map(|e| e.map(|f: i64| f as u32))?;
    let width_op: Option<u32> = call.get_flag("width").map(|e| e.map(|f: i64| f as u32))?;

    let mut height: Option<u32>;
    let mut width: Option<u32>;

    if let Some((w, h)) = term_size::dimensions() {
        // don't know why I need to scale this, but I do - I hope it works
        // as intended for other terminals.
        height = Some(height_op.unwrap_or((h as f32 * 1.7) as u32));
        width = Some(width_op.unwrap_or((w as f32 * 1.7) as u32));

        // textplot panics if either of these are below 32 units.
        if height.unwrap() < 32 {
            height = Some(32);
        }
        if width.unwrap() < 32 {
            width = Some(32);
        }
    } else {
        // we couldnt detect terminal size for some reason
        height = height_op;
        width = width_op;
    }

    let legend = call.has_flag("legend");
    let steps = call.has_flag("steps");
    let bars = call.has_flag("bars");
    let points = call.has_flag("points");
    let bins: Option<u32> = call.get_flag("bins").map(|e| e.map(|f: i64| f as u32))?;
    let title: Option<String> = call.get_flag("title")?;

    Ok(CliOpts {
        height_op: height,
        width_op: width,
        legend,
        steps,
        bars,
        points,
        bins,
        title,
    })
}

/// The shape of the plot. Default is `Shape::Lines`,
/// but also includes `Shape::Bars` and `Shape::Steps`.
fn chart_shape<'a>(
    steps: bool,
    bars: bool,
    points: bool,
    call: &EvaluatedCall,
    v: &'a [(f32, f32)],
) -> Result<Shape<'a>, LabeledError> {
    match (steps, bars, points) {
        (true, false, false) => Ok(Shape::Steps(v)),
        (false, true, false) => Ok(Shape::Bars(v)),
        (false, false, true) => Ok(Shape::Points(v)),
        (false, false, false) => Ok(Shape::Lines(v)),
        _ => Err(LabeledError {
            label: "Chart shape error".into(),
            msg:
                "Shape must be either steps or bars or points, not more than one. Check your flags!"
                    .into(),
            span: Some(call.head),
        }),
    }
}

/// Return the minimum and the maximum of a slice of `f32`.
fn min_max(series: &[f32]) -> (f32, f32) {
    let min = series
        .iter()
        .fold(std::f32::MAX, |accu, &x| if x < accu { x } else { accu });
    let max = series
        .iter()
        .fold(std::f32::MIN, |accu, &x| if x > accu { x } else { accu });
    (min, max)
}

impl Plotter {
    /// Plot a single list of numbers.
    fn plot(
        &self,
        call: &EvaluatedCall,
        input: &Value,
        plot_type: &str,
    ) -> Result<Value, LabeledError> {
        let CliOpts {
            height_op,
            width_op,
            legend,
            steps,
            bars,
            points,
            title,
            bins,
        } = parse_cli_opts(call)?;

        let max_x = width_op.unwrap_or(200);
        let max_y = height_op.unwrap_or(50);

        let values = input.as_list()?;

        let v: Result<Vec<(f32, f32)>, LabeledError> = values
            .iter()
            .enumerate()
            .map(|(i, e)| match e {
                Value::Int { val: _, span: _ } => Ok((i as f32, e.as_int()? as f32)),
                Value::Float { val: _, span: _ } => Ok((i as f32, e.as_f64()? as f32)),
                e => Err(LabeledError {
                    label: "Incorrect type supplied.".into(),
                    msg: format!("Got {}, need integer or float.", e.get_type()),
                    span: Some(call.head),
                }),
            })
            .collect();

        let mut min_max_x = {
            let x: Vec<f32> = v.clone().unwrap().iter().map(|e| e.0).collect();
            min_max(&x)
        };

        let chart_data = match PlotType::from(plot_type) {
            PlotType::Plot => v,
            PlotType::Hist => {
                let (min, max) = min_max(
                    &v.clone()
                        .unwrap()
                        .iter()
                        .map(|(_, e)| *e)
                        .collect::<Vec<f32>>(),
                );
                let hist_data = histogram(
                    &v.unwrap(),
                    min,
                    max,
                    bins.map(|e| e as usize).unwrap_or(20),
                );

                min_max_x = (min, max);

                Ok(hist_data)
            }
            PlotType::XYplot => Err(LabeledError {
                label: "Plot type error.".into(),
                msg: "Doesn't make sense to plot an xyplot with a single list of values.".into(),
                span: Some(call.head),
            }),
        };

        let mut chart = Chart::new(max_x, max_y, min_max_x.0, min_max_x.1)
            .lineplot(&chart_shape(steps, bars, points, call, &chart_data?)?)
            .to_string();

        if let Some(t) = title {
            chart = TAB.to_owned() + &t + "\n" + &chart;
        }
        chart = TAB.to_owned() + &chart.replace('\n', &format!("\n{}", TAB));

        if legend {
            chart += &format!("Line 1: {}", "---".white());
        }

        Ok(Value::String {
            val: chart,
            span: call.head,
        })
    }

    /// Plot a nested list of numbers.
    ///
    /// It's guaranteed when calling this function that
    /// the input is a nested list with each element of equal length
    /// and type (int/float)
    fn plot_nested(
        &self,
        call: &EvaluatedCall,
        input: &Value,
        plot_type: &str,
    ) -> Result<Value, LabeledError> {
        let CliOpts {
            height_op,
            width_op,
            legend,
            steps,
            bars,
            points,
            title,
            bins,
        } = parse_cli_opts(call)?;

        let max_x = width_op.unwrap_or(200);
        let max_y = height_op.unwrap_or(50);

        let values = input.as_list()?;
        if values.len() > 5 {
            return Err(LabeledError {
                label: "Nested list error.".into(),
                msg: "Nested list can't contain more than 5 inner lists.".into(),
                span: Some(call.head),
            });
        }

        let mut data = vec![];

        for val in values {
            let list = val.as_list()?;

            let v: Result<Vec<(f32, f32)>, LabeledError> = list
                .iter()
                .enumerate()
                .map(|(i, e)| match e {
                    Value::Int { val: _, span: _ } => Ok((i as f32, e.as_int()? as f32)),
                    Value::Float { val: _, span: _ } => Ok((i as f32, e.as_f64()? as f32)),
                    e => Err(LabeledError {
                        label: "Incorrect type supplied.".into(),
                        msg: format!("Got {}, need integer or float.", e.get_type()),
                        span: Some(call.head),
                    }),
                })
                .collect();

            let min_max_x = {
                let x: Vec<f32> = v.clone()?.iter().map(|e| e.0).collect();
                let y = if plot_type == "xyplot" {
                    let temp: Vec<f32> = v.clone()?.iter().map(|e| e.1).collect();
                    Some(min_max(&temp))
                } else {
                    None
                };
                (min_max(&x), y)
            };

            data.push((min_max_x, v?));
        }

        let (mut min, mut max) = 'minmax: {
            // only interested in the first list
            if plot_type == "xyplot" {
                let (_, xy_x) = &data[0].0;
                break 'minmax xy_x.unwrap();
            }
            let min_all: Vec<f32> = data.iter().map(|((e, _), _)| e.0).collect();
            let max_all: Vec<f32> = data.iter().map(|((e, _), _)| e.1).collect();

            let min = min_all.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let max = max_all.iter().max_by(|a, b| a.total_cmp(b)).unwrap();

            (min, *max)
        };

        // copying data structure again here but wanted to be explicit.
        let chart_data: Vec<Vec<(f32, f32)>> = match PlotType::from(plot_type) {
            PlotType::Plot => data.iter().map(|(_, e)| e.clone()).collect(),
            PlotType::Hist => {
                // we need to adjust the x axis for the histogram.
                let mut mins = 0.0;
                let mut maxs = 0.0;

                for (i, (_, el)) in data.iter().enumerate() {
                    let (min, max) = min_max(&el.iter().map(|(_, e)| *e).collect::<Vec<f32>>());
                    if i == 0 {
                        maxs = max;
                        mins = min;
                    } else {
                        if max > maxs {
                            maxs = max;
                        }
                        if min < mins {
                            mins = min;
                        }
                    }
                }

                let hist_data: Vec<Vec<(f32, f32)>> = data
                    .iter()
                    .map(|(_, e)| histogram(e, mins, maxs, bins.map(|e| e as usize).unwrap_or(20)))
                    .collect();

                (min, max) = (mins, maxs);

                hist_data
            }
            PlotType::XYplot => {
                // quick and dirty for the moment.
                if data.len() != 2 {
                    return Err(LabeledError {
                        label: "Wrong number of dimensions in xyplot.".into(),
                        msg: "xyplot requires a nested list of length 2.".into(),
                        span: Some(call.head),
                    });
                }
                let y: Vec<f32> = data[1].1.iter().map(|e| e.1).collect();
                let xy: Vec<(f32, f32)> = data[0].1.iter().map(|e| e.1).zip(y).collect();
                vec![xy]
            }
        };

        let mut chart = Chart::new(max_x, max_y, min, max);

        let charts = match chart_data.len() {
            // this is xyplot
            1 => chart
                .lineplot(&chart_shape(steps, bars, points, call, &chart_data[0])?)
                .to_string(),
            // this is plot/hist
            2 => chart
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[0])?,
                    COLORS[0],
                )
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[1])?,
                    COLORS[1],
                )
                .to_string(),
            3 => chart
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[0])?,
                    COLORS[0],
                )
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[1])?,
                    COLORS[1],
                )
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[2])?,
                    COLORS[2],
                )
                .to_string(),
            4 => chart
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[0])?,
                    COLORS[0],
                )
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[1])?,
                    COLORS[1],
                )
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[2])?,
                    COLORS[2],
                )
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[3])?,
                    COLORS[3],
                )
                .to_string(),
            5 => chart
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[0])?,
                    COLORS[0],
                )
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[1])?,
                    COLORS[1],
                )
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[2])?,
                    COLORS[2],
                )
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[3])?,
                    COLORS[3],
                )
                .linecolorplot(
                    &chart_shape(steps, bars, points, call, &chart_data[4])?,
                    COLORS[4],
                )
                .to_string(),
            _ => unreachable!(),
        };

        let mut final_chart = TAB.to_owned() + &charts.replace('\n', &format!("\n{}", TAB));

        if let Some(t) = title {
            final_chart = TAB.to_owned() + &t + "\n" + &final_chart;
        }

        if legend {
            for (l, (_, _)) in data.iter().enumerate() {
                let col: PixelColor = COLORS[l];
                final_chart += &format!("Line {}: {} ", l + 1, "---".color(col));
            }
        }

        Ok(Value::String {
            val: final_chart,
            span: call.head,
        })
    }
}

/// Get the type of a `Value`, and its length if it's a list.
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

    if let Some(_len) = first_len_op {
        // *should* always index + unwrap without panicking...
        let inner_type = l[0].as_list()?[0].get_type();
        match inner_type {
            Type::Float | Type::Int => (),
            _ => {
                return Err(LabeledError {
                    label: "Incorrect type.".into(),
                    msg: "Nested list elements not float or int.".into(),
                    span: Some(call.head),
                })
            }
        }
    }

    Ok((first_type.clone(), *first_len_op))
}

impl Plugin for Plotter {
    // Try and keep it one command with a few flags
    fn signature(&self) -> Vec<PluginSignature> {
        vec![
            // plot
            PluginSignature::build("plot")
                .usage("Render an ASCII plot from a list of values.")
                .named(
                    "width",
                    SyntaxShape::Number,
                    "The maximum width of the plot.",
                    None,
                )
                .named(
                    "height",
                    SyntaxShape::Number,
                    "The maximum height of the plot.",
                    None,
                )
                .named(
                    "title",
                    SyntaxShape::String,
                    "Provide a title to the plot.",
                    Some('t'),
                )
                .switch("legend", "Plot a tiny, maybe useful legend.", Some('l'))
                .switch("bars", "Change lines to bars.", Some('b'))
                .switch("steps", "Change lines to steps.", Some('s'))
                .switch("points", "Change lines to points.", Some('p'))
                .category(Category::Experimental),
            // histogram
            PluginSignature::build("hist")
                .usage("Render an ASCII histogram from a list of values.")
                .named(
                    "width",
                    SyntaxShape::Number,
                    "The maximum width of the plot.",
                    None,
                )
                .named(
                    "height",
                    SyntaxShape::Number,
                    "The maximum height of the plot.",
                    None,
                )
                .named(
                    "title",
                    SyntaxShape::String,
                    "Provide a title to the plot.",
                    Some('t'),
                )
                .named(
                    "bins",
                    SyntaxShape::Number,
                    "The number of bins in the histogram, default is 20.",
                    None,
                )
                .switch("legend", "Plot a tiny, maybe useful legend.", Some('l'))
                .switch("bars", "Change lines to bars.", Some('b'))
                .switch("steps", "Change lines to steps.", Some('s'))
                .category(Category::Experimental),
            // plot
            PluginSignature::build("xyplot")
                .usage("Render an ASCII xy plot from a list of values.")
                .named(
                    "width",
                    SyntaxShape::Number,
                    "The maximum width of the plot.",
                    None,
                )
                .named(
                    "height",
                    SyntaxShape::Number,
                    "The maximum height of the plot.",
                    None,
                )
                .named(
                    "title",
                    SyntaxShape::String,
                    "Provide a title to the plot.",
                    Some('t'),
                )
                .switch("legend", "Plot a tiny, maybe useful legend.", Some('l'))
                .switch("bars", "Change lines to bars.", Some('b'))
                .switch("steps", "Change lines to steps.", Some('s'))
                .switch("points", "Change lines to points.", Some('p'))
                .category(Category::Experimental),
        ]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        match name {
            "plot" | "hist" | "xyplot" => {
                // here we want to check what the input is.
                match input.as_list() {
                    Ok(list) => {
                        // so we have a list. what's in it? we need to check each inner value
                        if list.is_empty() {
                            return Err(LabeledError {
                                label: "No elements in the list.".into(),
                                msg: "Can't plot a zero element list.".into(),
                                span: Some(call.head)
                            })
                        }

                        let (value_type, list_len_op) = check_equality_of_list(list, call)?;

                        // if in fact we have a nested list
                        if let Some(_len) = list_len_op {
                            // we haven't implemented this yet
                            self.plot_nested(call, input, name)
                        } else {
                            // we have a normal plot, single list of numbers
                            match value_type {
                                Type::Float | Type::Int => self.plot(call, input, name),
                                e =>  Err(LabeledError {
                                    label: "Incorrect List type.".into(),
                                    msg: format!("List type is {}, but should be float or int.", e),
                                    span: Some(call.head)
                                })
                            }
                        }
                    },
                    Err(e) => return Err(LabeledError {
                        label: "Incorrect input type.".into(),
                        msg: format!("Input type should be a list: {}.", e),
                        span: Some(call.head)
                    }),
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
