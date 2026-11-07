# Noir

A private, black-and-white web browser written from scratch in Rust.
No Chromium. No Firefox engine. No AI. Just Noir.

**Version:** 0.0.2 beta (expect bugs, and the occasional ransom-note layout)

## What it does

- Opens web pages over HTTPS and shows text, headings, links and lists
- Reads CSS: colors, backgrounds, font sizes, bold, centered text and hidden elements, from style tags, linked stylesheets and inline styles
- Vertical tabs with open, close and switch
- Gaming mode: puts background tabs to sleep to free memory
- History and Settings behind a password gate
- Search from the address bar (Google or DuckDuckGo Lite, chosen in Settings)
- Black-and-white interface

## What it can't do yet

- Images
- JavaScript, so Google results, Gmail and YouTube are out for now
- Flexbox, grid and media queries
- Real box layout (padding, borders, widths)
- Real password encryption (the beta password is a placeholder)

## Roadmap

1. Window and UI shell (done)
2. Networking (basic HTTPS works)
3. HTML parser (basic)
4. CSS and layout (in progress)
5. Rendering (images, fonts)
6. Encrypted history with a real password
7. Gaming mode: auto-start when a game launches
8. JavaScript

## Getting a build

1. Open the Actions tab of this repo.
2. Pick "Build Noir" and press Run workflow.
3. When it turns green, open the run and download the Windows or Linux build from Artifacts.

Targets: Windows and Debian 13 (Trixie).

## Project layout

- src/main.rs: the whole browser (UI, networking, HTML and CSS parsing)
- Cargo.toml: dependencies (eframe and ureq)
- .github/workflows/build.yml: builds Windows and Linux versions

## Privacy

Noir has no accounts, no telemetry and no AI features. Your history stays on your machine.

## Status

Early beta. Built for learning and for fun. Pull requests are welcome, but please be nice to the CSS parser, it's trying its best.