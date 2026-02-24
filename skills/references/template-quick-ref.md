# Template Quick Reference

Compact reference of all Earl template shapes, field types, and options.

## Operation Shapes by Protocol

**HTTP** — fields are flat in operation block:

```hcl
operation {
  protocol = "http"
  method   = "GET"
  url      = "https://..."
  query    = { key = "value" }
  headers  = { Accept = "application/json" }
  auth     { kind = "bearer"; secret = "key" }
  body     { kind = "json"; value = { ... } }
}
```

**GraphQL** — nested `graphql` block:

```hcl
operation {
  protocol = "graphql"
  url      = "https://..."
  graphql {
    query     = "query { ... }"
    variables = { key = "value" }
  }
  auth { kind = "bearer"; secret = "key" }
}
```

**gRPC** — nested `grpc` block:

```hcl
operation {
  protocol   = "grpc"
  url        = "http://host:port"
  timeout_ms = 5000
  grpc {
    service             = "package.Service"
    method              = "Method"
    descriptor_set_file = "optional.fds.bin"
    body                = { field = "value" }
  }
}
```

**Bash** — nested `bash` block:

```hcl
operation {
  protocol = "bash"
  bash {
    script = "command here"
    sandbox {
      network          = false
      max_time_ms      = 30000
      max_output_bytes = 1048576
    }
  }
}
```

**SQL** — nested `sql` block:

```hcl
operation {
  protocol = "sql"
  sql {
    connection_secret = "db.url"
    query             = "SELECT * FROM t WHERE id = ?"
    params            = ["{{ args.id }}"]
    sandbox {
      read_only = true
      max_rows  = 100
    }
  }
}
```

## Auth Block Kinds

| Kind              | Fields                        | Use case                                 |
| ----------------- | ----------------------------- | ---------------------------------------- |
| `bearer`          | `secret`                      | API tokens (most REST/GraphQL APIs)      |
| `api_key`         | `secret`, `location`, `name`  | API key in header or query param         |
| `basic`           | `username`, `password_secret` | Basic HTTP auth                          |
| `o_auth2_profile` | `profile`                     | OAuth2 flows (configured in config.toml) |

## Body Block Kinds (HTTP only)

| Kind                | `value` type | Use case              |
| ------------------- | ------------ | --------------------- |
| `json`              | Object/map   | JSON request body     |
| `form_urlencoded`   | Object/map   | URL-encoded form data |
| `raw_text`          | String       | Raw text body         |
| `raw_bytes_base64`  | String       | Raw bytes (base64)    |

## Parameter Types

`string`, `integer`, `number`, `boolean`, `null`, `array`, `object`

## Result Decode Options

`auto`, `json`, `text`, `html`, `xml`, `binary`

## Result Extract Options

```hcl
extract = { json_pointer = "/data/items" }
```

## Template Expression Context

| Context     | Available in         | Example                   |
| ----------- | -------------------- | ------------------------- |
| `args.*`    | Everywhere           | `{{ args.query }}`        |
| `secrets.*` | Operation block only | `{{ secrets.myapi_key }}` |
| `result`    | Result block only    | `{{ result.total }}`      |

## HCL Functions

| Function                | Example                          |
| ----------------------- | -------------------------------- |
| `file("path")`          | Read a file relative to template |
| `base64encode("value")` | Base64 encode                    |
| `trimspace("value")`    | Trim whitespace                  |
