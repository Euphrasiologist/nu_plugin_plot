# `nu_plugin_plot`

A small nu plugin to plot a list as a line graph.

## Install

Not yet on crates.io, so you'll have to clone this repository. I assume you have Rust, and are inside a nushell instance.

```console
git clone https://github.com/Euphrasiologist/nu_plugin_plot
cd nu_plugin_plot

cargo build --release
register ./target/release/nu_plugin_plot

# test commands
plot -h
hist -h
xyplot -h
```

## Help

`plot`, `hist`, and `xyplot` have very similar helps, so I'll print out just plot here.

```console
Render an ASCII plot from a list of values.

Usage:
  > plot {flags} 

Flags:
  -h, --help - Display the help message for this command
  --width <Number> - The maximum width of the plot.
  --height <Number> - The maximum height of the plot.
  -t, --title <String> - Provide a title to the plot.
  -l, --legend - Plot a tiny, maybe useful legend.
  -b, --bars - Change lines to bars.
  -s, --steps - Change lines to steps.
  -p, --points - Change lines to points.
```

## Examples

```console
## basic 'plot'

# plot a single line
let one = (seq 0.0 0.01 20.0 | math sin)
$one | plot

# plot two lines
let two = (seq 1.0 0.01 21.0 | math sin)
[$one $two] | plot

# plot four lines with a legend and title
let three = (seq 2.0 0.01 22.0 | math sin)
let four = (seq 3.0 0.01 23.0 | math sin)

[$one $two $three $four] | plot -l -t "Four sine lines!"

# bivariate 'xyplot'
# input must be a two element nested list

# make a nice ellipse
[$one $two] | xyplot

# bivariate line plot
# diagonal dots!
[(seq 1 100) (seq 1 100 | reverse)] | xyplot -p

# plot histograms

# compare two uniform distributions
let r1 = (seq 1 100 | each { random integer ..30})
let r2 = (seq 1 100 | each { random integer ..30})

# -b for bars, otherwise you get lines by default
[$r1 $r2] | hist -b
# up the number of bins
[$r1 $r2] | hist -b --bins 50

# If you've got R installed (& Rscript)
# go crazy!
# forget ggplot!
let x = (Rscript -e "cat(dnorm(seq(-4, 4, length=100)))" | into string | split row ' ' | into decimal)
let y = (Rscript -e "cat(dnorm(seq(-3, 6, length=100)))" | into string | split row ' ' | into decimal)

[$x $y] | plot -bl -t "Two normal distributions"
```

## Features

Plot:

- [x] a single numeric list
- [x] a list of numeric lists
  - [x] with colour support
  - [x] with legend
  - [x] with title
- [x] scatter plots (as a list of two numeric lists)
- [x] histogram (list rendered as a bar chart)
- [ ] nested xyplot (i.e. multiple xyplots on the same plot...)
- [ ] records..?

Please help me make this better! Submit issues/PR's, happy to chat.

The color rendering inside nushell is slightly confusing - you may notice I've included my own modified copies of `textplots` and `drawille` in the source code. This is because their color rendering method was not working inside the plugin - I still don't know why.