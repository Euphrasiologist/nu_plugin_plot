# `nu_plugin_plot`

A small plugin to plot simple data in a workflow.

Currently implemented only for `List`s.

```console
# simple line graph
[1, 2, 3, 4, 5, 4, 3, 2, 1] | plot
# multiple line graphs on the same plot
# up to only 4/5 lines I think...
[
    [1, 2, 3, 4, 5, 4, 3, 2, 1], 
    [5, 4, 3, 2, 1, 2, 3, 4, 5]
] | plot

# might be nice to have a very simple xyplot too.
# would need to check that list length is 2 (and is a list of lists),
# and that each list is the same length.
[
    [1, 2, 3, 4, 5, 4, 3, 2, 1], 
    [5, 4, 3, 2, 1, 2, 3, 4, 5]
] | xyplot
```


