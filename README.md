# traefik-route

A lean CLI for managing Traefik custom domain routes on Coolify-hosted servers.

Coolify generates routing rules for its own sslip.io domains only. Getting your real domain (`myapp.com`) routed through Traefik requires dropping a YAML file in `/data/coolify/proxy/dynamic/`. This tool writes and manages those files.

## Install

```bash
cargo install --git https://github.com/madebymadhouse/traefik-route
```

Or download a pre-built binary from [releases](https://github.com/madebymadhouse/traefik-route/releases).

## Usage

```
traefik-route <COMMAND>

Commands:
  add     Add a domain -> port route
  remove  Remove a route by domain
  list    List all active routes
  dry     Preview config without writing
```

### Add a route

```bash
# Route myapp.com:3000 with HTTPS (Let's Encrypt)
traefik-route add myapp.com 3000

# Also route www.myapp.com
traefik-route add myapp.com 3000 --www

# HTTP only (no Let's Encrypt)
traefik-route add myapp.com 3000 --no-https
```

Traefik picks up the change instantly. No restart required.

### Preview without writing

```bash
traefik-route dry myapp.com 3000 --www
```

### List active routes

```bash
traefik-route list
```

### Remove a route

```bash
traefik-route remove myapp.com
```

### Custom directory

```bash
traefik-route --dir /custom/path/dynamic add myapp.com 3000
```

## What it generates

```yaml
http:
  routers:
    myapp-com-http:
      rule: Host(`myapp.com`) || Host(`www.myapp.com`)
      entryPoints: [http]
      service: myapp-com-svc
      priority: 10
    myapp-com-https:
      rule: Host(`myapp.com`) || Host(`www.myapp.com`)
      entryPoints: [https]
      service: myapp-com-svc
      priority: 10
      tls:
        certResolver: letsencrypt
  services:
    myapp-com-svc:
      loadBalancer:
        servers:
          - url: http://host.docker.internal:3000
        passHostHeader: true
```

The backend uses `host.docker.internal:<port>` rather than the container name. Coolify changes the container name on every deploy; the host port binding does not change. This means routes written by `traefik-route` survive redeploys without modification.

## Context

Part of the [coolify-vps-playbook](https://github.com/madebymadhouse/coolify-vps-playbook). See that repo for the full setup guide.

## License

MIT
