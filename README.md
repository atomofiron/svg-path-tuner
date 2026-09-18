# vector-path-tuner

`M 2 2 L 3 1 Z` <=> `M 8 8 L 12 4 Z` <=> `M 8 8 l 4 -4 z`  

## Usage

`$ vector-path-tuner [-s size] [-t r|a] [path ...]`

- a positional argument is an `.xml` file or a folder with them: a folder gives the `.xml` files in it,
  without recursion into nested folders, every other file in the folder is ignored

- `-s`, `--size` target viewport size of the files, `12` or `24x24`, optional, by default the
  viewport is kept as is. One number means a square target, so a `24x48` viewport with `-s 12`
  gives `scaleX = 12/24 = 0.5` and `scaleY = 12/48 = 0.25`, while `-s 24x24` gives `1` and `0.5`
- `-t`, `--target` target coordination: `r` relative or `a` absolute (also `relative` / `absolute`),
  required for files
- without files the path is read from stdin, no scaling happens (`-s` needs files, there is no
  viewport to compute the ratios from), `-t` limits the output to one form
- with files it scales `android:pathData` per axis, replaces `android:viewportWidth` and
  `android:viewportHeight` with the target size (arc radii too, arc flags and rotation stay as is)
  and rewrites every `.xml` file in place
- a skipped file is reported on stderr and left untouched, not a byte of it changes:
  - an `.xml` file without a `<vector>` tag at all — a `<shape>`, `<selector>` or bitmap drawable, the usual
    neighbours of the vectors in a `res/drawable` folder
  - a file that references `@string`: its path data lives in `strings.xml`, so the viewport alone must not be
    scaled, and the file is skipped before anything is tokenized
- coordinates are computed in exact decimal integer math and rounded to 4 decimals on output,
  so no floating point noise reaches the path

## Example
```
$ vector-path-tuner -t r                                                                                                                                              15:49:31
input path: M 8 8 L 12 4 Z
parts: M, 8, 8, L, 12, 4, Z  # just for info
relative: m8 8l4-4z
absolute: M8 8L12 4Z
input path:  # just press Enter to exit

$ vector-path-tuner -s 12 -t a ic_add.xml ic_remove.xml
ic_add.xml: 2 pathData, 2 viewport, scale 0.5x0.5
ic_remove.xml: 1 pathData, 2 viewport, scale 0.5x0.5

$ vector-path-tuner -s 12 -t a app/src/main/res/drawable
ic_add.xml: 2 pathData, 2 viewport, scale 0.5x0.5
ic_circle_shape.xml: no <vector> in the file, the file was skipped
```
