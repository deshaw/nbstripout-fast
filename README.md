# nbstripout-fast

[![PyPI version][pypi-image]][pypi-url] [![PyPI DM][pypi-dm-image]][pypi-url]
[![Github Actions Status][github-status-image]][github-status-url]

A much faster version of [nbstripout](https://github.com/kynan/nbstripout) by writing it in rust (of course).
This helps strip Jupyter Notebook output and metadata from notebooks. It is very useful as a git filter
and is highly configurable.

## Installation
```
pip install nbstripout-fast
```

Then replace nbstripout-fast with anywhere you use nbstripout.

## Key differences
1. While we mirrored most of nbstripout's API, we do not support every
nbstripout option.
2. There is no CLI option to install this in git for you
3. We support repository level settings in a `.git-nbconfig.yaml` file. Check out
our `examples`. On a high level, you can add a git filter in a sitewide/user level
and then allow each project to enforce consistent settings.

### Why Rust?

nbstripout is a excellent project, but the python startup and import time makes
its usage at scale a bit painful. While this means giving up on using nbconvert
under the hood and ensuring the notebook is the correct format, it does make things
up to 200x faster. This matters when you have a large number of files and git filter
is called sometimes more than once per file. Let's look at the data:

| Cells |  nbstripout |  nbstripout_fast |
|-------|-------------|------------|
| 1     |  0m0.266s   |   0m0.003s |
| 10    |  0m0.258s   |   0m0.003s |
| 100   |  0m0.280s   |   0m0.004s |
| 1000  |  0m0.372s   |   0m0.013s |
| 10000 |  0m1.649s   |   0m0.133s |

The table above shows a large overhead per notebook (mostly python startup time).
When you have 100 or more notebooks, nbstripout takes more than 40s while
nbstripout-fast takes only 1s!

## Developing
You can use cargo which will build + run the CLI:
```
cargo run -- -t examples/example.ipynb
```

You can also build with cargo and run the script with the full path:
```
cargo build # dev build - ./target/debug/nbstripout-fast
cargo build --release # release build - ./target/release/nbstripout-fast
```

Running unit tests:
maturin builds this repo to include pyo3 bindings by default. This allows
for us to have an extension python extension mode as well. As of today, we can't
have a binary and an extension, so we use the extension only for testing
([issue](https://github.com/PyO3/maturin/discussions/1006)).
```
pip install -e .
maturin develop
# Should output, this way you can use RUST_LOG=debug
in-venv pytest -rP
```

### Debugging
Use RUST_LOG=debug to debug script for example:
```
RUST_LOG=debug cargo run -- '--extra-keys "metadata.bar cell.baz" -t foo.ipynb'
```

## Releasing

Manylinux, macos, and windows wheels and sdist are built by github workflows.
Builds are triggered upon the creation of a pull request, creating a new
release, or with a manual workflow dispatch. The wheels and sdist are only
uploaded to PyPI when a new release is published. In order to create a new
release:

1. Create a commit updating the version in `Cargo.toml` and `CHANGELOG.md`, then create a git tag:
```bash
git tag vX.Y.Z
git push --tags
```
2. Draft a new release in github; select the tag that you just created.
3. Once the new release is created, the wheels and sdist will be built by a
   github workflow and then uploaded to PyPI automatically using the
   `PYPI_API_TOKEN` in the github secrets for the repository.

## History

This plugin was contributed back to the community by the [D. E. Shaw group](https://www.deshaw.com/).

<p align="center">
    <a href="https://www.deshaw.com">
       <img src="https://www.deshaw.com/assets/logos/blue_logo_417x125.png" alt="D. E. Shaw Logo" height="75" >
    </a>
</p>

## License

This project is released under a [BSD-3-Clause license](https://github.com/deshaw/nbstripout-fast/blob/master/LICENSE.txt).

We love contributions! Before you can contribute, please sign and submit this [Contributor License Agreement (CLA)](https://www.deshaw.com/oss/cla).
This CLA is in place to protect all users of this project.


[pypi-url]: https://pypi.org/project/nbstripout-fast
[pypi-image]: https://img.shields.io/pypi/v/nbstripout-fast
[pypi-dm-image]: https://img.shields.io/pypi/dm/nbstripout-fast
[github-status-image]: https://github.com/deshaw/nbstripout-fast/workflows/Build/badge.svg
[github-status-url]: https://github.com/deshaw/nbstripout-fast/actions?query=workflow%3ABuild
