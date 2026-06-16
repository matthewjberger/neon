# Neon Documentation

The book for [Neon](https://github.com/matthewjberger/neon), built with
[mdBook](https://rust-lang.github.io/mdBook/).

## Building

Install mdBook:

```sh
cargo install mdbook
```

Build and serve locally:

```sh
just serve     # or: mdbook serve --open
```

The book deploys to GitHub Pages on every push to `main`
(`.github/workflows/pages.yml`).

## License

Neon is dual-licensed under either:

- MIT License (http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 (http://www.apache.org/licenses/LICENSE-2.0)

at your option.
