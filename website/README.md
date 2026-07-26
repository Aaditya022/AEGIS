# AEGIS Website

This directory contains the AEGIS project website source.

## Quick Start

```bash
# If using a static site generator (e.g., Hugo, Jekyll)
# Follow the generator's setup instructions

# For plain HTML:
open index.html
```

## Pages

- `/` — Landing page with hero, features, architecture diagram
- `/docs/` — Full documentation
- `/docs/quickstart` — Getting started guide
- `/docs/architecture` — Architecture overview
- `/docs/integrations` — Framework integration guides
- `/docs/deployment` — K8s, Docker, bare-metal deployment
- `/docs/api` — API reference
- `/docs/security` — Security model
- `/docs/benchmarks` — Performance results
- `/blog/` — Announcements and updates
- `/community/` — Community resources

## Deployment

The website is deployed via GitHub Pages. Push to `main` triggers an automatic build and deploy.

## Build

```bash
# For Hugo-based site:
hugo server -D    # Development server
hugo              # Build to ./public
```

## License

Same as AEGIS project: Apache 2.0
