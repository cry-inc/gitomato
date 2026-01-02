# gitomato
[![Build Status](https://github.com/cry-inc/gitomato/workflows/CI/badge.svg)](https://github.com/cry-inc/gitomato/actions)
[![No Unsafe](https://img.shields.io/badge/unsafe-forbidden-brightgreen.svg)](https://doc.rust-lang.org/nomicon/meet-safe-and-unsafe.html)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Dependencies](https://deps.rs/repo/github/cry-inc/gitomato/status.svg)](https://deps.rs/repo/github/cry-inc/gitomato)

Simple and lightweight HTTP server for static web pages from git repositories with automated updates.
Create your own "GitHub Pages" for self-hosted web apps and similar use cases!

It does **not** support scripting or automatic transformations, every file from the git repository is served as-is.

## Features
- [x] Serve one or more pages from different git repositories.
- [x] Select a specific branch (or just use the default branch).
- [x] Very easy to configure and deploy.
- [x] Uses a shallow clone to save disk space and bandwidth.
- [x] All files are kept in memory to avoid disk IO when serving pages.
- [x] Can also serve only a subfolder of the repository.
- [x] Update pages automatically via regular update intervals.
- [x] Update pages instanly using optional web hooks.
- [x] Can also serve directory index pages (default is off).
- [x] Supports client-side caching using ETag based on git file hashes.
- [x] Compiled to a single statically linked executable.
- [x] Very small Docker image (less than 10 MB).
- [x] Prebuilt binaries and Docker images.
- [x] Runs out of the box on a Raspberry Pi.
- [x] Implemented in Rust (memory safe, fast, easy to build).

## Configuration
See `gitomato --help` for all global parameters.  
Run `gitomato` without any arguments for a quick start guide.

The simplest possible configuration of gitomato is this:  
`gitomato --page-git-repo=https://git.server.org/my-repo.git`

### Docker
You can also use the pre-built Docker images to deploy gitomato.
When using Docker, the recommended way of configuration are environment variables.
Here an example configuration with two separate pages:
```
docker run --rm -p 8080:8080 \
    -e PAGE0_GIT_REPO="https://user:pass@server.org/my-repo.git" \
    -e PAGE0_GIT_REF="master" \
    -e PAGE0_PREFIX="/page0/" \
    -e PAGE0_GIT_SUBFOLDER="htdocs/" \
    -e PAGE0_AUTO_LIST=true \
    -e PAGE0_UPDATE_SECRET="secr3t" \
    -e PAGE0_MAX_BYTES=10000000 \
    -e PAGE1_GIT_REPO="https://server.com/another-repo.git" \
    -e PAGE1_PREFIX="/page1/" \
    ghcr.io/cry-inc/gitomato
```

#### Available Docker Tags
The following tags are available:
* `latest` (latest stable release)
* `develop` (last development build from main branch)
* `<major>`, e.g. `1` (latest stable release from that major version)
* `<major>.<minor>`, e.g. `1.1` (latest stable release from that minor version)
* `<major>.<minor>.<patch>`, e.g. `1.1.0` (a specific release)

### Automatic Updates
There are two ways to update your pages when the git repository changes.
The default option is an automatic update interval.
The server will check the git repos in configured intervals for any changes.
If you run the default configuration, the update check interval is 300 seconds.
You can change this using the global configuration option `--interval` or the environment variable `INTERVAL`.

The second option is an web hook that can be called by your [git forge](https://en.wikipedia.org/wiki/Forge_(software)) whenever new data is pushed to git.
This will result in immediate updates, but requires extra setup.
You have to enable the update secret for your page, for example using `--page-update-secret=123`.
This enables a HTTP GET endpoint below your page root at `/update/123`.
You can then use this secret URL to set up the web hook in your git forge.

### HTTPS
This application only exposes an HTTP server. Support for HTTPS is not included.
Its recommended to use an reverse proxy like [Caddy](https://caddyserver.com/) in front of this application to add HTTPS support.

## Why?
Why not just use GitHub Pages?

I wanted a solution that works for any git repository accessible over HTTP(S).
And I wanted to be able to selfhost all parts.
Additionally, other existing solutions like [GitLab Pages](https://docs.gitlab.com/user/project/pages/), [pages-server](https://codeberg.org/Codeberg/pages-server) or [git-pages](https://codeberg.org/git-pages/git-pages) are too big and complex for my own use cases.
