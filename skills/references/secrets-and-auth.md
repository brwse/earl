# Secrets and Authentication

Earl stores secrets in the OS keychain (Apple Keychain, Windows Credential Manager, Linux Secret Service) and supports multiple authentication patterns.

## Managing Secrets

```bash
# Set a secret (interactive prompt)
earl secrets set myapi.token

# Set a secret from stdin (for piping)
echo "sk-abc123" | earl secrets set myapi.token --stdin

# View a secret
earl secrets get myapi.token

# List all secrets
earl secrets list

# Delete a secret
earl secrets delete myapi.token
```

## Auth Block Kinds

Use an `auth` block inside `operation` to attach credentials to requests:

```hcl
# Bearer token
auth {
  kind   = "bearer"
  secret = "myapi.token"
}

# Basic auth
auth {
  kind            = "basic"
  username        = "myuser"
  password_secret = "myapi.password"
}

# OAuth2 profile (uses config.toml profile)
auth {
  kind    = "o_auth2_profile"
  profile = "myservice"
}
```

## Referencing Secrets in Templates

In `auth` blocks, use the `secret` field with the dotted key name:

```hcl
auth {
  kind   = "bearer"
  secret = "github.token"
}
```

In template expressions (e.g., headers or bash scripts), use `secrets.*` with underscores:

```hcl
headers = {
  X-API-Key = "{{ secrets.myapi_key }}"
}
```

Note: dots in secret keys become underscores in template expressions (`myapi.key` becomes `secrets.myapi_key`).

## OAuth2 Profiles

For OAuth2 flows, configure a profile in `~/.config/earl/config.toml`:

```toml
[auth.profiles.myservice]
flow = "device_code"
client_id = "your-client-id"
token_url = "https://auth.example.com/token"
device_authorization_url = "https://auth.example.com/device"
scopes = ["read", "write"]
```

Supported flows: `device_code`, `auth_code_pkce`, `client_credentials`.

Log in:

```bash
earl auth login myservice
```

Check status:

```bash
earl auth status myservice
```

Then reference it in templates:

```hcl
auth {
  kind    = "o_auth2_profile"
  profile = "myservice"
}
```

## Annotations

Declare which secrets a command needs in `annotations`:

```hcl
annotations {
  mode    = "read"
  secrets = ["myapi.token"]
}
```

This tells Earl (and MCP consumers) which secrets are required before the command can run.
