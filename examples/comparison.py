#!/usr/bin/env python

import re
import subprocess
import tempfile
from pathlib import Path

import nbformat
from nbconvert.preprocessors import ExecutePreprocessor

HERE = Path(__file__).parent.resolve()

NBSTRIPOUT_FAST = str((HERE / "../target/release/nbstripout-fast").resolve())
NBSTRIPOUT = "nbstripout"
CODE_CELL_RATIO = 10  # 10:1


def create(num_cells):
    nb = nbformat.v4.new_notebook()
    for i in range(num_cells):
        if i % CODE_CELL_RATIO == 0:
            nb.cells.append(nbformat.v4.new_markdown_cell("### I am cell {}".format(i)))
        else:
            nb.cells.append(nbformat.v4.new_code_cell("x = {}\nx".format(i)))
    return nb


def run(nb):
    ep = ExecutePreprocessor(timeout=600, kernel_name="python3")
    ep.preprocess(nb, {"metadata": {"path": "./"}})
    return nb


def main():
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        count_plus_filenames = []
        for num_cells in [1, 10, 100, 1_000, 10_000]:
            nb = run(create(num_cells))
            filename = tmpdir / f"generated-{num_cells}-cells.ipynb"
            count_plus_filenames.append((num_cells, filename))
            with open(filename, "w") as f:
                nbformat.write(nb, f)

        print("{:<7} {:<12} {:<12}".format("Cells", "nbstripout", "nbstripout_fast"))
        for num_cells, file in count_plus_filenames:
            times = []
            for cmd in [NBSTRIPOUT, NBSTRIPOUT_FAST]:
                # emulate git filter by outputting to stdout
                output = subprocess.check_output(
                    f"time {cmd} {file} -t > /dev/null",
                    stderr=subprocess.STDOUT,
                    universal_newlines=True,
                    shell=True,
                )
                real_time = re.match(r"real\s+(\w+\.\w+s)", output.strip()).group(1)
                times.append(real_time)
            print("{:<7} {:<12} {:<12}".format(num_cells, times[0], times[1]))


if __name__ == "__main__":
    main()
