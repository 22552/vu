# vu

`vu` is the deliberately small, download-only part of a Python package workflow.
It fetches release artifacts from the PyPI JSON API, verifies their SHA-256 digest,
and never creates an environment, installs a package, or resolves dependencies.

```bash
cargo install --path .
vu requests==2.32.3 -d vendor/
vu numpy==2.3.0 --all -d vendor/
vu pydantic --no-binary -d vendor/
```

By default `vu` downloads one source distribution (sdist), which is portable and
unambiguous. `--all` downloads every non-yanked artifact for the selected release;
this is useful for building an offline wheelhouse. `--no-binary` restricts output to
sdists. Use an exact version for reproducible builds.

## Non-goals

- Dependency solving (`pip download` still does that)
- Wheel compatibility selection for another OS/Python target
- Installing packages or managing environments

Those features turn it into a package manager. `vu` is meant to stay a verified
artifact fetcher.

## License

MIT
