# svg-path-tuner

`M 2 2 L 3 1 Z` <=> `M 8 8 L 12 4 Z` <=> `M 8 8 l 4 -4 z`  

## Usabe

`$ svg-path-tuner [-s scale|/scale] [-t r|a] [file.xml ...]`

- `-s`, `--scale` scale ratio, `2` or `/2`, optional, `1` by default
- `-t`, `--target` target coordination: `r` relative or `a` absolute (also `relative` / `absolute`),
  required for files
- without files the path is read from stdin, `-t` limits the output to one form
- with files it scales `android:pathData`, `android:viewportWidth` and `android:viewportHeight`
  (arc radii too, arc flags and rotation stay as is) and rewrites every `.xml` file in place
- coordinates are computed in exact decimal integer math and rounded to 4 decimals on output,
  so no floating point noise reaches the path

## Example
```
$ svg-path-tuner -s /4                                                                                                                                              15:49:31
input path: M 8 8 L 12 4 Z
parts: M, 8, 8, L, 12, 4, Z  # just for info
m2 2l1-1z    # relative coordinates
M2 2L3 1Z    # absolute coordinates
input path:  # just press Enter to exit

$ svg-path-tuner -s 2 -t a ic_add.xml ic_remove.xml
ic_add.xml: 2 pathData, 2 viewport, scale 2
ic_remove.xml: 1 pathData, 2 viewport, scale 2
```
