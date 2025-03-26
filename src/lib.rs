//! A small crate to plot an ASCII
//! representation of a List data type from nushell
//!
//! Three commands are supplied.
//! - `plot` plots a 1-dimensional numeric list/nested list
//! - `hist` plots a 1-dimensional numeric list/nested list
//! - `xyplot` plots a 2-dimensional numeric list (nested list with length == 2)

use nu_plugin::{EvaluatedCall, Plugin, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Signature, SyntaxShape, Type, Value};
pub mod color_plot;

use color_plot::drawille::PixelColor;
use color_plot::textplots::{utils::histogram, Chart, ColorPlot, Plot, Shape};
use owo_colors::OwoColorize;


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

    let legend = call.has_flag("legend")?;
    let steps = call.has_flag("steps")?;
    let bars = call.has_flag("bars")?;
    let points = call.has_flag("points")?;
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
        _ => Err(LabeledError::new("Shape must be either steps or bars or points, not more than one. Check your flags!").with_label("Chart shape error", call.head)),
    }
}

/// Check the chart shape is Okay. If not returns an error.
fn check_chart_shape<'a>(
    steps: bool,
    bars: bool,
    points: bool,
    call: &EvaluatedCall,
) -> Result<(), LabeledError> {
    match (steps, bars, points) {
        (true, false, false) => Ok(()),
        (false, true, false) => Ok(()),
        (false, false, true) => Ok(()),
        (false, false, false) => Ok(()),
        _ => Err(LabeledError::new("Shape must be either steps or bars or points, not more than one. Check your flags!").with_label("Chart shape error", call.head)),
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
        return Err(LabeledError::new("Can't plot a list of multiple types.").with_label("Type differences.", call.head) );
    }

    let first_len_op = &len_ops[0];
    let check_len_pass = len_ops.iter().all(|e| e == first_len_op);

    if !check_len_pass {
        return Err(LabeledError::new("Can't plot a list of differing length lists.").with_label("List length differences.", call.head));
    }

    if let Some(_len) = first_len_op {
        // *should* always index + unwrap without panicking...
        let inner_type = l[0].as_list()?[0].get_type();
        match inner_type {
            Type::Float | Type::Int => (),
            _ => {
                return Err(LabeledError::new("Nested list elements not float or int.").with_label("Incorrect type.", call.head));
            }
        }
    }

    Ok((first_type.clone(), *first_len_op))
}

pub struct PluginPlot {}

struct CommandPlot;
struct CommandHist;
struct CommandXyplot;

impl Plugin for PluginPlot {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }
    fn commands(&self) -> Vec<Box<dyn nu_plugin::PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(CommandPlot), Box::new(CommandHist), Box::new(CommandXyplot)
        ]
    }
}

trait Plotter {
    fn plot(
        &self,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError>;
    fn plot_nested(
        &self,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError>;
}

impl Plotter for CommandPlot {
    fn plot(
        &self,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let CliOpts {
            height_op,
            width_op,
            legend,
            steps,
            bars,
            points,
            title,
            bins: _,
        } = parse_cli_opts(call)?;

        let max_x = width_op.unwrap_or(200);
        let max_y = height_op.unwrap_or(50);

        let values = input.as_list()?;

        let v: Result<Vec<(f32, f32)>, LabeledError> = values
            .iter()
            .enumerate()
            .map(|(i, e)| match e {
                Value::Int { .. } => Ok((i as f32, e.as_int()? as f32)),
                Value::Float { .. } => Ok((i as f32, e.as_float()? as f32)),
                e => Err(LabeledError::new(format!("Got {}, need integer or float.", e.get_type())).with_label("Incorrect type supplied", call.head)),
            })
            .collect();

        let min_max_x = {
            let x: Vec<f32> = v.clone().unwrap().iter().map(|e| e.0).collect();
            min_max(&x)
        };

        let chart_data = v;

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

        Ok(Value::string(chart, call.head))
    }

    fn plot_nested<'a>(
        &self,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let CliOpts {
            height_op,
            width_op,
            legend,
            steps,
            bars,
            points,
            title,
            bins: _,
        } = parse_cli_opts(call)?;

        let max_x = width_op.unwrap_or(200);
        let max_y = height_op.unwrap_or(50);

        let values = input.as_list()?;
        if values.len() > 5 {
            return Err(LabeledError::new("Nested list can't contain more than 5 inner lists.").with_label("Nested list error.", call.head));
        }

        let mut data = vec![];

        for val in values {
            let list = val.as_list()?;

            let v: Result<Vec<(f32, f32)>, LabeledError> = list
                .iter()
                .enumerate()
                .map(|(i, e)| match e {
                    Value::Int { .. } => Ok((i as f32, e.as_int()? as f32)),
                    Value::Float { .. } => Ok((i as f32, e.as_float()? as f32)),
                    e => Err(LabeledError::new(format!("Got {}, need integer or float.", e.get_type())).with_label("Incorrect type supplied.", call.head)),
                })
                .collect();

            let min_max_x = {
                let x: Vec<f32> = v.clone()?.iter().map(|e| e.0).collect();
                let y: Option<Vec<f32>> = None;
                (min_max(&x), y)
            };

            data.push((min_max_x, v?));
        }

        let min_all: Vec<f32> = data.iter().map(|((e, _), _)| e.0).collect();
        let max_all: Vec<f32> = data.iter().map(|((e, _), _)| e.1).collect();

        let min = min_all.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max = *max_all.iter().max_by(|a, b| a.total_cmp(b)).unwrap();

        // copying data structure again here but wanted to be explicit.
        let chart_data: Vec<Vec<(f32, f32)>> = data.iter().map(|(_, e)| e.clone()).collect();

        // let shapes = chart_data.into_iter().map(|data| chart_shape(steps, bars, points, call, &data));
        check_chart_shape(steps, bars, points, call)?;
        let shapes: Vec<Shape> = (&chart_data)
            .iter()
            .map(|data| chart_shape(steps, bars, points, call, data).unwrap())
            .collect();
        let charts = (&shapes).iter()
            .enumerate()
            .fold(&mut Chart::new(max_x, max_y, min, max), |chart, (i, shape)| {
                chart.linecolorplot(shape, COLORS[i])
            })
            .to_string();

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

        Ok(Value::string(final_chart, call.head))
    }
}


impl SimplePluginCommand for CommandPlot {
    type Plugin = PluginPlot;

    fn name(&self) -> &str {
        "plot"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("plot")
            .description("Render an ASCII plot from a list of values.")
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
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Render an ASCII plot from a list of values."
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &nu_plugin::EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        match input.as_list() {
            Ok(list) => {
                if list.is_empty() {
                    return Err(LabeledError::new("Can't plot a zero element list.").with_label( "No elements in the list.", call.head));
                }
                let (value_type, list_len_op) = check_equality_of_list(list, call)?;

                // if in fact we have a nested list
                if let Some(_len) = list_len_op {
                    // we haven't implemented this yet
                    self.plot_nested(call, input)
                } else {
                    // we have a normal plot, single list of numbers
                    match value_type {
                        Type::Float | Type::Int => self.plot(call, input),
                        e =>  Err(LabeledError::new(format!("List type is {}, but should be float or int.", e)).with_label("Incorrect List type.", call.head)),
                    }
                }
            },
            Err(e) => Err(LabeledError::new(format!("Input type should be a list: {}.", e)).with_label( "Incorrect input type.", call.head)),
        }
    }
}

impl Plotter for CommandHist {
    fn plot(
        &self,
        call: &EvaluatedCall,
        input: &Value,
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
                Value::Int { .. } => Ok((i as f32, e.as_int()? as f32)),
                Value::Float { .. } => Ok((i as f32, e.as_float()? as f32)),
                e => Err(LabeledError::new(format!("Got {}, need integer or float.", e.get_type())).with_label("Incorrect type supplied", call.head)),
            })
            .collect();

        let (min, max) = min_max(
            &v.clone()
                .unwrap()
                .iter()
                .map(|(_, e)| *e)
                .collect::<Vec<f32>>(),
        );
        let chart_data: Vec<(f32, f32)> = histogram(
            &v.unwrap(),
            min,
            max,
            bins.map(|e| e as usize).unwrap_or(20),
        );
        let min_max_x = (min, max);


        let mut chart = Chart::new(max_x, max_y, min_max_x.0, min_max_x.1)
            .lineplot(&chart_shape(steps, bars, points, call, &chart_data)?)
            .to_string();

        if let Some(t) = title {
            chart = TAB.to_owned() + &t + "\n" + &chart;
        }
        chart = TAB.to_owned() + &chart.replace('\n', &format!("\n{}", TAB));

        if legend {
            chart += &format!("Line 1: {}", "---".white());
        }

        Ok(Value::string(chart, call.head))
    }

    fn plot_nested(
        &self,
        call: &EvaluatedCall,
        input: &Value,
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
            return Err(LabeledError::new("Nested list can't contain more than 5 inner lists.").with_label("Nested list error.", call.head));
        }

        let mut data = vec![];

        for val in values {
            let list = val.as_list()?;

            let v: Result<Vec<(f32, f32)>, LabeledError> = list
                .iter()
                .enumerate()
                .map(|(i, e)| match e {
                    Value::Int { .. } => Ok((i as f32, e.as_int()? as f32)),
                    Value::Float { .. } => Ok((i as f32, e.as_float()? as f32)),
                    e => Err(LabeledError::new(format!("Got {}, need integer or float.", e.get_type())).with_label("Incorrect type supplied.", call.head)),
                })
                .collect();

            let x: Vec<f32> = v.clone()?.iter().map(|e| e.0).collect();
            let y: Option<Vec<f32>> = None;
            let min_max_x = (min_max(&x), y);

            data.push((min_max_x, v?));
        }

        // copying data structure again here but wanted to be explicit.
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
        let (min, max) = (mins, maxs);

        let hist_data: Vec<Vec<(f32, f32)>> = data
            .iter()
            .map(|(_, e)| histogram(e, mins, maxs, bins.map(|e| e as usize).unwrap_or(20)))
            .collect();

        check_chart_shape(steps, bars, points, call)?;
        let shapes: Vec<Shape> = (&hist_data)
            .iter()
            .map(|data| chart_shape(steps, bars, points, call, data).unwrap())
            .collect();
        let charts = (&shapes).iter()
            .enumerate()
            .fold(&mut Chart::new(max_x, max_y, min, max), |chart, (i, shape)| {
                chart.linecolorplot(shape, COLORS[i])
            })
            .to_string();

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

        Ok(Value::string(final_chart, call.head))
    }
}

impl SimplePluginCommand for CommandHist {
    type Plugin = PluginPlot;

    fn name(&self) -> &str {
        "hist"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("hist")
            .description("Render an ASCII histogram from a list of values.")
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
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Render an ASCII histogram from a list of values."
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &nu_plugin::EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        match input.as_list() {
            Ok(list) => {
                if list.is_empty() {
                    return Err(LabeledError::new("Can't plot a zero element list.").with_label( "No elements in the list.", call.head));
                }
                let (value_type, list_len_op) = check_equality_of_list(list, call)?;

                // if in fact we have a nested list
                if let Some(_len) = list_len_op {
                    // we haven't implemented this yet
                    self.plot_nested(call, input)
                } else {
                    // we have a normal plot, single list of numbers
                    match value_type {
                        Type::Float | Type::Int => self.plot(call, input),
                        e =>  Err(LabeledError::new(format!("List type is {}, but should be float or int.", e)).with_label("Incorrect List type.", call.head)),
                    }
                }
            },
            Err(e) => Err(LabeledError::new(format!("Input type should be a list: {}.", e)).with_label( "Incorrect input type.", call.head)),
        }
    }
}

impl Plotter for CommandXyplot {
    fn plot(
        &self,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        Err(LabeledError::new( "Doesn't make sense to plot an xyplot with a single list of values.").with_label("Plot type error.", call.head))
    }

    fn plot_nested(
        &self,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let CliOpts {
            height_op,
            width_op,
            legend,
            steps,
            bars,
            points,
            title,
            bins: _,
        } = parse_cli_opts(call)?;

        let max_x = width_op.unwrap_or(200);
        let max_y = height_op.unwrap_or(50);

        let values = input.as_list()?;
        if values.len() > 5 {
            return Err(LabeledError::new("Nested list can't contain more than 5 inner lists.").with_label("Nested list error.", call.head));
        }

        let mut data = vec![];

        for val in values {
            let list = val.as_list()?;

            let v: Result<Vec<(f32, f32)>, LabeledError> = list
                .iter()
                .enumerate()
                .map(|(i, e)| match e {
                    Value::Int { .. } => Ok((i as f32, e.as_int()? as f32)),
                    Value::Float { .. } => Ok((i as f32, e.as_float()? as f32)),
                    e => Err(LabeledError::new(format!("Got {}, need integer or float.", e.get_type())).with_label("Incorrect type supplied.", call.head)),
                })
                .collect();

            let min_max_x = {
                let x: Vec<f32> = v.clone()?.iter().map(|e| e.0).collect();
                let temp: Vec<f32> = v.clone()?.iter().map(|e| e.1).collect();
                let y = Some(min_max(&temp));
                (min_max(&x), y)
            };

            data.push((min_max_x, v?));
        }
        if data.len() != 2 {
            return Err(LabeledError::new("xyplot requires a nested list of length 2.").with_label( "Wrong number of dimensions in xyplot.", call.head));
        }

        let (min, max) = {
            // only interested in the first list
            let (_, xy_x) = &data[0].0;
            xy_x.unwrap()
        };

        let y: Vec<f32> = data[1].1.iter().map(|e| e.1).collect();
        let xy: Vec<(f32, f32)> = data[0].1.iter().map(|e| e.1).zip(y).collect();
        let chart_data = vec![xy];

        let mut chart = Chart::new(max_x, max_y, min, max);

        let charts = chart
            .lineplot(&chart_shape(steps, bars, points, call, &chart_data[0])?)
            .to_string();


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

        Ok(Value::string(final_chart, call.head))
    }
}

impl SimplePluginCommand for CommandXyplot {
    type Plugin = PluginPlot;

    fn name(&self) -> &str {
        "xyplot"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("xyplot")
            .description("Render an ASCII xy plot from a list of values.")
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
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Render an ASCII xy plot from a list of values."
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &nu_plugin::EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        match input.as_list() {
            Ok(list) => {
                if list.is_empty() {
                    return Err(LabeledError::new("Can't plot a zero element list.").with_label( "No elements in the list.", call.head));
                }
                let (value_type, list_len_op) = check_equality_of_list(list, call)?;

                // if in fact we have a nested list
                if let Some(_len) = list_len_op {
                    // we haven't implemented this yet
                    self.plot_nested(call, input)
                } else {
                    // we have a normal plot, single list of numbers
                    match value_type {
                        Type::Float | Type::Int => self.plot(call, input),
                        e =>  Err(LabeledError::new(format!("List type is {}, but should be float or int.", e)).with_label("Incorrect List type.", call.head)),
                    }
                }
            },
            Err(e) => Err(LabeledError::new(format!("Input type should be a list: {}.", e)).with_label( "Incorrect input type.", call.head)),
        }
    }
}

