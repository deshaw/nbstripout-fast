from setuptools import setup, find_packages
from pathlib import Path

HERE = Path(__file__).parent.resolve()

long_description = (HERE / "README.md").read_text()

setup(
    long_description=long_description,
    long_description_content_type="text/markdown",
)
