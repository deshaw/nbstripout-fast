# nbstripout-fast

[![PyPI version][pypi-image]][pypi-url] [![PyPI DM][pypi-dm-image]][pypi-url]
[![Github Actions Status][github-status-image]][github-status-url]

A much faster version of [nbstripout](https://github.com/kynan/nbstripout) by writing it in rust (of course).
This helps strip Jupyter Notebook output and metadata from notebooks. It is very useful as a git filter
and is highly configurable.

## Installation
```bash
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

## Examples

In the following, we consider two use cases of `nbstripout-fast`: using it as a **pre-commit hook** and as a **Git filter**.

### Pre-commit hook example

`nbstripout-fast` can be used as a [pre-commit](https://pre-commit.com/) to automatically clean outputs from Jupyter notebooks before each commit. Importantly, pre-commit hooks can also be integrated into [GitHub Actions](https://docs.github.com/en/actions) for enhanced CI/CD workflows as described in the documentation of [pre-commit action](https://github.com/pre-commit/action). Note that unlike Git filters, pre-commit hooks modify your local files.

1. **Install `pre-commit`**
    ```bash
    pip install pre-commit
    ```
2. Add the `nbstripout-fast` hook to the `.pre-commit-config.yaml` file at the root of your repository: 
    ```yaml
    repos:
      - repo: https://github.com/deshaw/nbstripout-fast
        rev: v1.1.1
        hooks:
          - id: nbstripout-fast
            name: nbstripout-fast
            entry: nbstripout-fast
            types: [jupyter]
            language: python
    ```
    > **Note:** Even though `nbstripout-fast` is implemented in `Rust`, using `language: python` in your `.pre-commit-config.yaml` can be beneficial, as using `language: rust` forces pre-commit to build `nbstripout-fast` from source, which can fail on some platforms due to missing Python headers or mismatched toolchains.

3. **Configure nbstripout-fast**

   Create a `.git-nbconfig.yaml` file at the root of your repository to configure `nbstripout-fast`, e.g.
	```yaml
	nbstripout_fast:
	  keep_count: false
	  keep_output: false
	  drop_empty_cells: true
	  extra_keys: []
	  keep_keys: []
	```

4. **Install the pre-commit hooks**
    ```bash
    pre-commit install
    ```

### Git filter example
This example illustrates how `nbstripout-fast` can be used to automatically clean Jupyter notebooks using Git filters (see e.g. [Git Attributes](https://git-scm.com/book/en/v2/Customizing-Git-Git-Attributes)). This keeps your repository clean by removing unnecessary output and clutter, while preserving your local working version. The benefits are minimised diffs and reduced repository size.

1. **Install `nbstripout-fast`** as described above.
2. **Configure nbstripout-fast** (see example above)
3. **Set Git Attributes**

   Create a `.gitattributes` file at the root of your repository if it doesn't yet exist and add this line:
	```bash
	*.ipynb filter=jupyter
	```
	 This instructs Git to use a custom filter named "jupyter" on all `.ipynb` files.
4. **Configure the `jupyter` Filter**

   Run these commands in your terminal to configure the "jupyter" filter:
	```bash
	git config filter.jupyter.clean nbstripout-fast
	git config filter.jupyter.smudge cat
	```
- `clean`: This filter runs `nbstripout-fast` when adding notebooks to the version that is checked out, i.e. the clean version.
- `smudge`: This filter runs `cat` when checking out notebooks, ensuring your local (smudged) version remains unmodified.
  Git filters transform files at the time of checkout and commit.
4. **Reapply Cleaning to Existing Notebooks (Optional)**

   If you already have Jupyter notebooks tracked by Git, you can reapply the cleaning process to them:
	```bash
	git add --renormalize . git commit -m "Cleaned Jupyter notebooks"
	```

## Stripping specific cell outputs

To strip cell outputs that match a regular expression, the `--strip-regex`
option can be used in combination with `--keep-output`. For example, to remove
cell outputs that only contain a notebook widget:

```bash
nbstripout-fast --keep-output --strip-regex "^Output\(\)$"
```

or to remove completed tqdm progress bars:

```bash
nbstripout-fast --keep-output --strip-regex "100%.*"
```

See the [documentation for `regex`](https://docs.rs/regex/latest/regex/) for
information about supported regex syntax.

## Developing
You can use cargo which will build + run the CLI:
```bash
cargo run -- -t examples/example.ipynb
```

You can also build with cargo and run the script with the full path:
```bash
cargo build # dev build - ./target/debug/nbstripout-fast
cargo build --release # release build - ./target/release/nbstripout-fast
```

Running unit tests:
maturin builds this repo to include pyo3 bindings by default. This allows
for us to have an extension python extension mode as well. As of today, we can't
have a binary and an extension, so we use the extension only for testing
([issue](https://github.com/PyO3/maturin/discussions/1006)).
```bash
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
