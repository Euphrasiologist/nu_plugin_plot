//! A PERSONAL FORK OF 'textplots' VERSION 0.8.0 WITH
//! SUPPORT FOR ANSI COLORS IN NUSHELL.
//!
//! Terminal plotting library for using in CLI applications.
//! Should work well in any unicode terminal with monospaced font.
//!
//! It is inspired by [TextPlots.jl](https://github.com/sunetos/TextPlots.jl) which is inspired by [Drawille](https://github.com/asciimoo/drawille).
//!
//! Currently it features only drawing line plots on Braille canvas, but could be extended
//! to support other canvas and chart types just like [UnicodePlots.jl](https://github.com/Evizero/UnicodePlots.jl)
//! or any other cool terminal plotting library.
//!
//! Contributions are very much welcome!
//!
//! # Usage
//! ```toml
//! [dependencies]
//! textplots = "0.6"
//! ```
//!
//! ```rust
//! use textplots::{Chart, Plot, Shape};
//!
//! println!("y = sin(x) / x");
//!
//! Chart::default()
//!     .lineplot(&Shape::Continuous(Box::new(|x| x.sin() / x)))
//!     .display();
//! ```
//!
//! It will display something like this:
//!
//! <img src="https://github.com/loony-bean/textplots-rs/blob/master/doc/demo.png?raw=true"/>
//!
//! Default viewport size is 120 x 60 points, with X values ranging from -10 to 10.
//! You can override the defaults calling `new`.
//!
//! ```rust
//! use textplots::{Chart, Plot, Shape};
//!
//! println!("y = cos(x), y = sin(x) / 2");
//!
//! Chart::new(180, 60, -5.0, 5.0)
//!     .lineplot(&Shape::Continuous(Box::new(|x| x.cos())))
//!     .lineplot(&Shape::Continuous(Box::new(|x| x.sin() / 2.0)))
//!     .display();
//! ```
//!
//! <img src="https://github.com/loony-bean/textplots-rs/blob/master/doc/demo2.png?raw=true"/>
//!
//! You could also plot series of points. See [Shape](enum.Shape.html) and [examples](https://github.com/loony-bean/textplots-rs/tree/master/examples) for more details.
//!
//! <img src="https://github.com/loony-bean/textplots-rs/blob/master/doc/demo3.png?raw=true"/>

pub mod scale;
pub mod utils;

use super::drawille::Canvas as BrailleCanvas;
use super::drawille::PixelColor;
use scale::Scale;
use std::cmp;
use std::default::Default;
use std::f32;

/// How the chart will do the ranging on axes
#[derive(PartialEq)]
enum ChartRangeMethod {
    /// Automatically ranges based on input data
    AutoRange,
    /// Has a fixed range between the given min & max
    FixedRange,
}

/// Controls the drawing.
pub struct Chart<'a> {
    /// Canvas width in points.
    width: u32,
    /// Canvas height in points.
    height: u32,
    /// X-axis start value.
    xmin: f32,
    /// X-axis end value.
    xmax: f32,
    /// Y-axis start value (potentially calculated automatically).
    ymin: f32,
    /// Y-axis end value (potentially calculated automatically).
    ymax: f32,
    /// The type of y axis ranging we'll do
    y_ranging: ChartRangeMethod,
    /// Collection of shapes to be presented on the canvas.
    shapes: Vec<(&'a Shape<'a>, Option<PixelColor>)>,
    /// Underlying canvas object.
    canvas: BrailleCanvas,
}

/// Specifies different kinds of plotted data.
pub enum Shape<'a> {
    /// Real value function.
    Continuous(Box<dyn Fn(f32) -> f32 + 'a>),
    /// Points of a scatter plot.
    Points(&'a [(f32, f32)]),
    /// Points connected with lines.
    Lines(&'a [(f32, f32)]),
    /// Points connected in step fashion.
    Steps(&'a [(f32, f32)]),
    /// Points represented with bars.
    Bars(&'a [(f32, f32)]),
}

/// Provides an interface for drawing plots.
pub trait Plot<'a> {
    /// Draws a [line chart](https://en.wikipedia.org/wiki/Line_chart) of points connected by straight line segments.
    fn lineplot(&'a mut self, shape: &'a Shape) -> &'a mut Chart<'a>;
}

/// Provides an interface for drawing colored plots.
pub trait ColorPlot<'a> {
    /// Draws a [line chart](https://en.wikipedia.org/wiki/Line_chart) of points connected by straight line segments using the specified color
    fn linecolorplot(&'a mut self, shape: &'a Shape, color: PixelColor) -> &'a mut Chart<'a>;
}

impl<'a> Default for Chart<'a> {
    fn default() -> Self {
        Self::new(120, 60, -10.0, 10.0)
    }
}

impl<'a> Chart<'a> {
    /// Creates a new `Chart` object.
    ///
    /// # Panics
    ///
    /// Panics if `width` or `height` is less than 32.
    pub fn new(width: u32, height: u32, xmin: f32, xmax: f32) -> Self {
        if width < 32 {
            panic!("width should be more then 32, {} is provided", width);
        }

        if height < 32 {
            panic!("height should be more then 32, {} is provided", height);
        }

        Self {
            xmin,
            xmax,
            ymin: f32::INFINITY,
            ymax: f32::NEG_INFINITY,
            y_ranging: ChartRangeMethod::AutoRange,
            width,
            height,
            shapes: Vec::new(),
            canvas: BrailleCanvas::new(width, height),
        }
    }

    /// Creates a new `Chart` object with fixed y axis range.
    ///
    /// # Panics
    ///
    /// Panics if `width` or `height` is less than 32.
    pub fn new_with_y_range(
        width: u32,
        height: u32,
        xmin: f32,
        xmax: f32,
        ymin: f32,
        ymax: f32,
    ) -> Self {
        if width < 32 {
            panic!("width should be more then 32, {} is provided", width);
        }

        if height < 32 {
            panic!("height should be more then 32, {} is provided", height);
        }

        Self {
            xmin,
            xmax,
            ymin,
            ymax,
            y_ranging: ChartRangeMethod::FixedRange,
            width,
            height,
            shapes: Vec::new(),
            canvas: BrailleCanvas::new(width, height),
        }
    }

    /// Displays bounding rect.
    fn borders(&mut self) {
        let w = self.width;
        let h = self.height;

        self.vline(0);
        self.vline(w);
        self.hline(0);
        self.hline(h);
    }

    /// Draws vertical line.
    fn vline(&mut self, i: u32) {
        if i <= self.width {
            for j in 0..=self.height {
                if j % 3 == 0 {
                    self.canvas.set(i, j);
                }
            }
        }
    }

    /// Draws horizontal line.
    fn hline(&mut self, j: u32) {
        if j <= self.height {
            for i in 0..=self.width {
                if i % 3 == 0 {
                    self.canvas.set(i, self.height - j);
                }
            }
        }
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&mut self) -> String {
        self.figures();
        self.axis();

        let mut frame = self.canvas.frame();
        if let Some(idx) = frame.find('\n') {
            frame.insert_str(idx, &format!(" {0:.1}", self.ymax));
            frame.push_str(&format!(
                " {0:.1}\n{1: <width$.1}{2:.1}\n",
                self.ymin,
                self.xmin,
                self.xmax,
                width = (self.width as usize) / 2 - 3
            ));
        }
        frame
    }

    /// Prints canvas content.
    pub fn display(&mut self) {
        println!("{}", self.to_string());
    }

    /// Prints canvas content with some additional visual elements (like borders).
    pub fn nice(&mut self) {
        self.borders();
        self.display();
    }

    /// Show axis.
    pub fn axis(&mut self) {
        let x_scale = Scale::new(self.xmin..self.xmax, 0.0..self.width as f32);
        let y_scale = Scale::new(self.ymin..self.ymax, 0.0..self.height as f32);

        if self.xmin <= 0.0 && self.xmax >= 0.0 {
            self.vline(x_scale.linear(0.0) as u32);
        }
        if self.ymin <= 0.0 && self.ymax >= 0.0 {
            self.hline(y_scale.linear(0.0) as u32);
        }
    }

    // Show figures.
    pub fn figures(&mut self) {
        for (shape, color) in &self.shapes {
            let x_scale = Scale::new(self.xmin..self.xmax, 0.0..self.width as f32);
            let y_scale = Scale::new(self.ymin..self.ymax, 0.0..self.height as f32);

            // translate (x, y) points into screen coordinates
            let points: Vec<_> = match shape {
                Shape::Continuous(f) => (0..self.width)
                    .filter_map(|i| {
                        let x = x_scale.inv_linear(i as f32);
                        let y = f(x);
                        if y.is_normal() {
                            let j = y_scale.linear(y).round();
                            Some((i, self.height - j as u32))
                        } else {
                            None
                        }
                    })
                    .collect(),
                Shape::Points(dt) | Shape::Lines(dt) | Shape::Steps(dt) | Shape::Bars(dt) => dt
                    .iter()
                    .filter_map(|(x, y)| {
                        let i = x_scale.linear(*x).round() as u32;
                        let j = y_scale.linear(*y).round() as u32;
                        if i <= self.width && j <= self.height {
                            Some((i, self.height - j))
                        } else {
                            None
                        }
                    })
                    .collect(),
            };

            // display segments
            match shape {
                Shape::Continuous(_) | Shape::Lines(_) => {
                    for pair in points.windows(2) {
                        let (x1, y1) = pair[0];
                        let (x2, y2) = pair[1];
                        if let Some(color) = color {
                            self.canvas.line_colored(x1, y1, x2, y2, *color);
                        } else {
                            self.canvas.line(x1, y1, x2, y2);
                        }
                    }
                }
                Shape::Points(_) => {
                    for (x, y) in points {
                        self.canvas.set(x, y);
                    }
                }
                Shape::Steps(_) => {
                    for pair in points.windows(2) {
                        let (x1, y1) = pair[0];
                        let (x2, y2) = pair[1];

                        if let Some(color) = color {
                            self.canvas.line_colored(x1, y2, x2, y2, *color);
                            self.canvas.line_colored(x1, y1, x1, y2, *color);
                        } else {
                            self.canvas.line(x1, y2, x2, y2);
                            self.canvas.line(x1, y1, x1, y2);
                        }
                    }
                }
                Shape::Bars(_) => {
                    for pair in points.windows(2) {
                        let (x1, y1) = pair[0];
                        let (x2, y2) = pair[1];

                        if let Some(color) = color {
                            self.canvas.line_colored(x1, y2, x2, y2, *color);
                            self.canvas.line_colored(x1, y1, x1, y2, *color);
                            self.canvas.line_colored(x1, self.height, x1, y1, *color);
                            self.canvas.line_colored(x2, self.height, x2, y2, *color);
                        } else {
                            self.canvas.line(x1, y2, x2, y2);
                            self.canvas.line(x1, y1, x1, y2);
                            self.canvas.line(x1, self.height, x1, y1);
                            self.canvas.line(x2, self.height, x2, y2);
                        }
                    }
                }
            }
        }
    }

    /// Return the frame.
    pub fn frame(&self) -> String {
        self.canvas.frame()
    }

    fn rescale(&mut self, shape: &Shape) {
        // rescale ymin and ymax
        let x_scale = Scale::new(self.xmin..self.xmax, 0.0..self.width as f32);

        let ys: Vec<_> = match shape {
            Shape::Continuous(f) => (0..self.width)
                .filter_map(|i| {
                    let x = x_scale.inv_linear(i as f32);
                    let y = f(x);
                    if y.is_normal() {
                        Some(y)
                    } else {
                        None
                    }
                })
                .collect(),
            Shape::Points(dt) | Shape::Lines(dt) | Shape::Steps(dt) | Shape::Bars(dt) => dt
                .iter()
                .filter_map(|(x, y)| {
                    if *x >= self.xmin && *x <= self.xmax {
                        Some(*y)
                    } else {
                        None
                    }
                })
                .collect(),
        };

        let ymax = *ys
            .iter()
            .max_by(|x, y| x.partial_cmp(y).unwrap_or(cmp::Ordering::Equal))
            .unwrap_or(&0.0);
        let ymin = *ys
            .iter()
            .min_by(|x, y| x.partial_cmp(y).unwrap_or(cmp::Ordering::Equal))
            .unwrap_or(&0.0);

        self.ymin = f32::min(self.ymin, ymin);
        self.ymax = f32::max(self.ymax, ymax);
    }
}

impl<'a> ColorPlot<'a> for Chart<'a> {
    fn linecolorplot(&'a mut self, shape: &'a Shape, color: PixelColor) -> &'a mut Chart<'a> {
        self.shapes.push((shape, Some(color)));
        if self.y_ranging == ChartRangeMethod::AutoRange {
            self.rescale(shape);
        }
        self
    }
}

impl<'a> Plot<'a> for Chart<'a> {
    fn lineplot(&'a mut self, shape: &'a Shape) -> &'a mut Chart<'a> {
        self.shapes.push((shape, None));
        if self.y_ranging == ChartRangeMethod::AutoRange {
            self.rescale(shape);
        }
        self
    }
}
