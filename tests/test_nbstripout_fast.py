import json
from copy import deepcopy

import nbformat
import pytest
from nbconvert.preprocessors import ExecutePreprocessor
from nbstripout_fast import stripout

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
        nbformat.v4.new_code_cell("from ipywidgets import Output; o = Output(); o"),
        nbformat.v4.new_code_cell("with o: print('hi')"),
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
    strip_regex=None,
):
    if extra_keys is None:
        extra_keys = DEFAULT_EXTRA_KEYS
    content = stripout(
        nbformat.v4.writes(nb),
        keep_output=keep_output,
        keep_count=keep_count,
        extra_keys=extra_keys,
        drop_empty_cells=drop_empty_cells,
        strip_regex=strip_regex,
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
    assert len(stripped_notebook.cells[-3]["outputs"]) == 1


def test_keep_count():
    stripped_notebook = _stripout_helper(executed_nb, keep_count=True)

    assert all(len(cell.get("outputs", [])) == 0 for cell in stripped_notebook.cells)
    # A bit harder than all as cells without source don't have counts
    assert stripped_notebook.cells[-3]["execution_count"] == 3


def test_drop_empty_cells():
    stripped_notebook = _stripout_helper(executed_nb, drop_empty_cells=True)

    copied_nb = deepcopy(clean_nb)
    copied_nb.cells.pop(-4)  # The empty cell
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
    copied_nb.cells.pop(-4)  # The empty cell
    copied_nb
    assert copied_nb == stripped_notebook


@pytest.mark.parametrize(
    "keep_output",
    [
        True,
        False,
    ]
)
@pytest.mark.parametrize(
    ("strip_regex", "regex_matches"),
    [
        (None, False),
        (r"Output\(\)", True),
        (r"Output", True),
        (r"Output.*", True),
        (r"what?", False),
        (r"utput.*", True),
        (r".*utput.*", True),
        (r"put*", True),
        (r".*put.*", True),
        (r"put\(\)", True),
        (r"put\(\)$", True),
        (r"^put\(\)$", False),
        (r"Output$", False),
    ]
)
def test_regex(strip_regex, regex_matches, keep_output):
    """Check that outputs matching the regex get stripped."""
    stripped_notebook = _stripout_helper(executed_nb, strip_regex=strip_regex, keep_output=keep_output)
    assert len(executed_nb.cells[-2].outputs) > 0

    if regex_matches:
        # If there's a regex match, outputs get stripped regardless of keep_output
        assert len(stripped_notebook.cells[-2].outputs) == 0
    else:
        if keep_output:
            assert len(stripped_notebook.cells[-2].outputs) == 1
        else:
            assert len(stripped_notebook.cells[-2].outputs) == 0


@pytest.mark.parametrize(
    "keep_output",
    [
        True,
        False,
    ]
)
def test_no_regex(keep_output):
    """Check that outputs get stripped if no regex is specified."""
    stripped_notebook = _stripout_helper(executed_nb, keep_output=keep_output)
    assert len(executed_nb.cells[-2].outputs) > 0
    assert len(stripped_notebook.cells[-2].outputs) == (1 if keep_output else 0)


@pytest.mark.parametrize(
    "keep_output",
    [
        True,
        False,
    ]
)
@pytest.mark.parametrize(
    ("strip_regex", "n_outputs_matched"),
    [
        (None, [0, 0, 0, 0]),
        (r"Output\(\)", [0, 0, 1, 1]),
        (r"Output", [0, 0, 1, 1]),
        (r"Output.*", [0, 0, 1, 1]),
        (r"what?", [0, 0, 0, 0]),
        (r"utput.*", [0, 0, 1, 1]),
        (r".*utput.*", [0, 0, 1, 1]),
        (r"put*", [0, 0, 1, 1]),
        (r".*put.*", [0, 0, 1, 1]),
        (r"put\(\)", [0, 0, 1, 1]),
        (r"put\(\)$", [0, 0, 1, 1]),
        (r"^put\(\)$", [0, 0, 0, 0]),
        (r"Output$", [0, 0, 0, 0]),
        (r"100%", [0, 1, 0, 0]), # the tqdm percentage
        (r"[06]/6", [0, 1, 1, 0]),  # the tqdm counter 0/6 and 6/6
    ]
)
def test_regex2(strip_regex, n_outputs_matched, keep_output, widget_notebook):
    """Test that stripping via regex targets individual outputs, not output groups."""
    stripped = _stripout_helper(
        widget_notebook, strip_regex=strip_regex, keep_output=keep_output
    )

    # Check that the original notebook contains expected outputs
    n_outputs = [0, 1, 2, 2]
    for i, n_output in enumerate(n_outputs):
        assert len(widget_notebook['cells'][i]['outputs']) == n_output

    # If keep_output == True, all outputs will be kept except those targeted
    # by the regex. Otherwise, all outputs get stripped regardless of whether
    # they are hit by the regex or not.
    for i, (n_output, n_matched) in enumerate(zip(n_outputs, n_outputs_matched)):
        assert len(stripped["cells"][i]["outputs"]) == (
            n_output - n_matched if keep_output else 0
        )
