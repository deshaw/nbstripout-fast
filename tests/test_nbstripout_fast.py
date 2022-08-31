from nbstripout_fast import stripout
import nbformat
from nbconvert.preprocessors import ExecutePreprocessor
from copy import deepcopy
import json

# Copied from main.rs
DEFAULT_EXTRA_KEYS = [
    "metadata.signature",
    "metadata.widgets",
    "cell.metadata.collapsed",
    "cell.metadata.ExecuteTime",
    "cell.metadata.execution",
    "cell.metadata.heading_collapsed",
    "cell.metadata.hidden",
    "cell.metadata.scrolled",
] + [
    "metadata.language_info"
]  # This is added by executing the notebook


def create_notebook():
    nb = nbformat.v4.new_notebook()
    nb.metadata.kernelspec = {
        "display_name": "Python 3",
        "language": "python",
        "name": "python3",
    }
    nb.cells = [
        nbformat.v4.new_markdown_cell("# Welcome to my notebook"),
        nbformat.v4.new_code_cell("1 + 1"),
        nbformat.v4.new_code_cell("x = 2\nx +1"),
        nbformat.v4.new_markdown_cell("## A section"),
        nbformat.v4.new_code_cell(""),
        nbformat.v4.new_code_cell("x += 3\nx"),
    ]
    return nb


def run_nb(nb):
    copy = deepcopy(nb)
    ep = ExecutePreprocessor(timeout=600, kernel_name="python3")
    ep.preprocess(copy)
    return copy


def _stripout_helper(
    nb,
    *,
    keep_output=False,
    keep_count=False,
    extra_keys=None,
    drop_empty_cells=False,
):
    if extra_keys is None:
        extra_keys = DEFAULT_EXTRA_KEYS
    content = stripout(
        nbformat.v4.writes(nb),
        keep_output=keep_output,
        keep_count=keep_count,
        extra_keys=extra_keys,
        drop_empty_cells=drop_empty_cells,
    )
    return nbformat.v4.reads(content)


clean_nb = None
executed_nb = None


def setup_module(module):
    global clean_nb
    global executed_nb

    clean_nb = create_notebook()
    executed_nb = run_nb(clean_nb)


def test_strip_all():
    stripped_notebook = _stripout_helper(executed_nb)

    assert clean_nb == stripped_notebook


def test_keep_output():
    stripped_notebook = _stripout_helper(executed_nb, keep_output=True)

    assert all(
        cell.get("execution_count", None) is None for cell in stripped_notebook.cells
    )
    assert len(stripped_notebook.cells[-1]["outputs"]) == 1


def test_keep_count():
    stripped_notebook = _stripout_helper(executed_nb, keep_count=True)

    assert all(len(cell.get("outputs", [])) == 0 for cell in stripped_notebook.cells)
    # A bit harder than all as cells without source don't have counts
    assert stripped_notebook.cells[-1]["execution_count"] == 3


def test_drop_empty_cells():
    stripped_notebook = _stripout_helper(executed_nb, drop_empty_cells=True)

    copied_nb = deepcopy(clean_nb)
    copied_nb.cells.pop(-2)  # The empty cell
    copied_nb
    assert copied_nb == stripped_notebook


def test_keep_output_tag():
    copied_executed_nb = deepcopy(executed_nb)
    copied_executed_nb.cells[1]["metadata"]["tags"] = ["keep_output"]
    stripped_notebook = _stripout_helper(
        copied_executed_nb, keep_output=False, keep_count=False
    )

    assert sum(len(cell.get("outputs", [])) for cell in stripped_notebook.cells) == 1


def test_extra_keys():
    stripped_notebook = _stripout_helper(
        executed_nb, extra_keys=DEFAULT_EXTRA_KEYS + ["cell.id"]
    )

    assert all("id" not in cell for cell in stripped_notebook.cells)


def test_source_as_strings():
    stripped_notebook = nbformat.v4.reads(
        stripout(
            # nbformat.write turns source from String to List by default
            # Both are valid so we want to test both
            json.dumps(executed_nb),
            keep_output=False,
            keep_count=False,
            extra_keys=DEFAULT_EXTRA_KEYS,
            drop_empty_cells=True,
        )
    )

    copied_nb = deepcopy(clean_nb)
    copied_nb.cells.pop(-2)  # The empty cell
    copied_nb
    assert copied_nb == stripped_notebook
