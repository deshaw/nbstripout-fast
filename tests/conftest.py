import pathlib
import pytest
import nbformat


@pytest.fixture
def widget_notebook():
    with open(pathlib.Path(__file__).parent / 'test_notebook.ipynb') as f:
        return nbformat.v4.reads(f.read())
