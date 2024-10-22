# Licensing

We use [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) to audit our dependencies for
licensing compliance. This is checked on CI, but you also have the tool made available in the
`nix develop` devshell (see [the contributing guide](./CONTRIBUTING.md) for more details).

To check license compatibility after adding a new dependency you can run:

```sh
cargo deny check licenses
```

Or, alternatively, to check for any banned items (licenses, CVEs, dependencies, and so on), you can
run:

```sh
cargo deny check
```

This check is run on CI as a requirement for merge, so even if you forget to run it locally you will
have the peace of mind that you won't be able to merge code using incompatible licenses.
