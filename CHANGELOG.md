# Changelog

## [0.1.0] - 2026-05-20

### Added
- Initial release
- CLI for managing Traefik custom domain routes on Coolify-hosted servers
- `add` command to create route config files (with optional www and HTTPS/Let's Encrypt)
- `list` command to show all active routes in the dynamic config directory
- `remove` command to delete a route by domain
- `dry` command to preview config output without writing to disk
