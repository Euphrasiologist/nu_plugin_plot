# `nu_plugin_plot`

A small nu plugin to plot a list as a line graph.

## Help

```console
Render an ASCII plot from a list of values.

Usage:
  > plot {flags}

Flags:
  -h, --help - Display the help message for this command
  -x, --max-x <Number> - The maximum width of the plot.
  -y, --max-y <Number> - The maximum height of the plot.
  -t, --title <String> - Provide a title to the plot.
  -l, --legend - Plot a tiny, maybe useful legend.
```

## Examples

```console
# plot a single line
let one = (seq 0.0 0.01 20.0 | math sin)
$one | plot

# plot two lines
let two = (seq 1.0 0.01 21.0 | math sin)
[$one $two] | plot

# plot four lines with a legend and title
let three = (seq 2.0 0.01 22.0 | math sin)
let four = (seq 3.0 0.01 23.0 | math sin)

[$one $two $three $four] | plot -l -t "Four sin lines!"

```

## Features

Plot:

- [x] a single numeric list
- [x] a list of numeric lists
  - [x] with colour support
  - [x] with legend
  - [x] with title
- [ ] scatter plots (as a list of two numeric lists)
- [ ] histogram (list rendered as a bar chart)
- [ ] records..?