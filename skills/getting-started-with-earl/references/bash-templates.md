# Bash Templates

Use Bash when running shell commands, CLI tools, or local scripts.

## Template Skeleton

```hcl
version = 1
provider = "tools"

command "disk_usage" {
  title       = "Disk Usage"
  summary     = "Check disk usage for a path"
  description = "Reports disk usage using du"

  annotations {
    mode = "read"
  }

  param "path" {
    type        = "string"
    required    = true
    description = "Path to check"
  }

  operation {
    protocol = "bash"

    bash {
      script = "du -sh {{ args.path }}"

      sandbox {
        network      = false
        max_time_ms  = 30000
      }
    }
  }

  result {
    decode = "text"
    output = "{{ result }}"
  }
}
```

## Key Fields

| Field | Required | Description |
|---|---|---|
| `bash.script` | Yes | Shell command to execute (supports `{{ args.* }}`) |
| `bash.sandbox.network` | No | Allow network access (default: false) |
| `bash.sandbox.max_time_ms` | No | Timeout in milliseconds (default: from config) |
| `bash.sandbox.max_output_bytes` | No | Max output size (default: from config) |

**Note:** Bash uses a nested `bash` block inside `operation`.

## Sandbox

Bash commands run in a sandbox by default:
- **No network access** unless `network = true`
- **Configurable timeout** via `max_time_ms`
- **Output limits** via `max_output_bytes`

Global sandbox defaults can be set in `~/.config/earl/config.toml`:

```toml
[sandbox]
bash_max_time_ms = 60000
bash_max_output_bytes = 1048576
bash_allow_network = false
```

## Multi-line Scripts

Use HCL heredoc syntax for longer scripts:

```hcl
bash {
  script = <<-EOT
    echo "Checking system info..."
    uname -a
    df -h
    free -m 2>/dev/null || vm_stat
  EOT
}
```

## Auth

Bash templates typically do not use auth blocks. If you need credentials in a script, reference secrets directly:

```hcl
bash {
  script = "curl -H 'Authorization: Bearer {{ secrets.myapi_token }}' https://api.example.com/data"
}
```

Set the secret: `earl secrets set myapi.token`

Note that `secrets.*` uses underscores in template expressions (e.g., `secrets.myapi_token`), while the secret key uses dots (e.g., `myapi.token`).
