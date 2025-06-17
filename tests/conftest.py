import pathlib
import pytest
import nbformat


@pytest.fixture
def widget_notebook():
    """A sample notebook containing some widgets and other output.

    Added from https://github.com/deshaw/nbstripout-fast/pull/21.
    """
    with open(pathlib.Path(__file__).parent / 'test_notebook.ipynb') as f:
        return nbformat.v4.reads(f.read())
