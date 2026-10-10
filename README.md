<div align="center">
  <img alt="gray-browser" src="assets/icon.svg" width="120" height="120" />
  <h1>gray-browser</h1>
  <p><strong>Headless-Chromium fetch, screenshot, and PDF tools — with a no-browser HTTP fallback for plain pages.</strong></p>
  <p>
    <a href="https://gray.alignment.id">Website</a> ·
    <a href="https://github.com/vstaln/gray-browser">Source</a> ·
    <a href="https://github.com/vstaln/gray">gray</a>
  </p>
  <p>
    <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-1c1c20?style=flat-square&labelColor=0a0a0b" /></a>
    <a href="https://www.rust-lang.org"><img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-rust-1c1c20?style=flat-square&labelColor=0a0a0b&logo=rust&logoColor=d4a373" /></a>
    <img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-1c1c20?style=flat-square&labelColor=0a0a0b&color=7aa2f7" />
  </p>
</div>

<br/>
```bash
gray plugin install browser
```

`gray-browser` hands the agent a real browser surface: render a page and
read the text, shoot a screenshot, or save it as PDF.

## Tools

| Tool | What it does |
|---|---|
| `browser_fetch` | GET a URL and return readable text. Uses headless chromium `--dump-dom` when a browser is installed (JS-rendered); falls back to a plain HTTP GET + tag-strip for server-rendered pages when it isn't. Params: `url`, `raw` (keep HTML), `max_chars`. |
| `browser_shot` | Full-page PNG screenshot via headless chromium. Params: `url`, `out`, `width`, `height`. |
| `browser_pdf` | Print the page to PDF. Params: `url`, `out`. |

Every chromium invocation runs under a **per-call watchdog** that kills
stalls instead of hanging the turn. Missing chromium is reported as a
clear install hint, not a crash (`chromium`, `google-chrome`, `brave`,
…are probed in order).

## Build from source

```sh
cargo build --release
gray plugin check ./target/release/browser
```

## License

MIT
