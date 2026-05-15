# klauncher

A blazing fast app launcher with tasteful UI

![klauncher Screenshot](Screenshot%20from%202026-05-15%2015-09-16.png)

## Overview

klauncher is a Linux application launcher inspired by Tuna for macOS. It provides a fast, keyboard-driven way to find and launch applications, files, and more — all through a clean, minimal interface.

## Features

- **Fuzzy Search** — Find apps instantly with partial or non-contiguous letter matching
- **Fast & Lightweight** — Built with Rust and Tauri for minimal resource usage
- **Keyboard Driven** — Navigate and launch without touching the mouse
- **Learning Rankings** — Frequently used apps rise to the top over time
- **Extensible** — Designed to support files, clipboard history, smart links, and custom scripts

## Tech Stack

| Layer    | Technology            |
| -------- | --------------------- |
| Frontend | React, TypeScript, Vite |
| Backend  | Rust, Tauri 2         |
| Storage  | SQLite                |

## Getting Started

### Prerequisites

- Node.js & npm
- Rust toolchain

### Install Dependencies

```bash
npm install
```

### Run in Development Mode

```bash
npm run tauri dev
```

### Build for Production

```bash
npm run tauri build
```
