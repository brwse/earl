# SQL Templates

Use SQL when querying databases (PostgreSQL, MySQL, SQLite, etc.).

## Template Skeleton

```hcl
version = 1
provider = "analytics"

command "recent_orders" {
  title       = "Recent Orders"
  summary     = "Fetch recent orders from the database"
  description = "Queries the orders table with a configurable limit"

  annotations {
    mode    = "read"
    secrets = ["analytics.db_url"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    description = "Max rows to return"
    default     = 10
  }

  operation {
    protocol = "sql"

    sql {
      connection_secret = "analytics.db_url"
      query             = "SELECT id, customer, total, created_at FROM orders ORDER BY created_at DESC LIMIT ?"
      params            = ["{{ args.limit }}"]

      sandbox {
        read_only = true
        max_rows  = 100
      }
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} orders"
  }
}
```

## Key Fields

| Field | Required | Description |
|---|---|---|
| `sql.connection_secret` | Yes | Secret key containing the database connection URL |
| `sql.query` | Yes | SQL query with `?` placeholders for parameters |
| `sql.params` | No | Array of parameter values (supports `{{ args.* }}`) |
| `sql.sandbox.read_only` | No | Force read-only mode (default: true) |
| `sql.sandbox.max_rows` | No | Maximum rows returned |

**Note:** SQL uses a nested `sql` block inside `operation`.

## Critical: HCL/Jinja Interaction

HCL parsing happens BEFORE Jinja template rendering. This means all `{{ }}` expressions must be valid HCL tokens.

**Correct — string-wrapped params:**
```hcl
params = ["{{ args.limit }}"]
```

**Wrong — bare expression (invalid HCL):**
```hcl
params = [{{ args.limit }}]
```

The string-wrapped version works because Earl's `render_string_value` auto-parses pure Jinja expressions as JSON, so `"{{ args.limit }}"` correctly becomes a number when `args.limit` is an integer.

## Connection URL Format

The connection secret should contain a standard database URL:

- **PostgreSQL:** `postgres://user:pass@host:5432/dbname`
- **MySQL:** `mysql://user:pass@host:3306/dbname`
- **SQLite:** `sqlite:///path/to/database.db`

Set it: `earl secrets set analytics.db_url --stdin` (then paste the URL)

## Sandbox

SQL queries run with safety defaults:
- **Read-only by default** (`read_only = true`) — prevents accidental writes
- **Row limits** — cap output size with `max_rows`

To allow writes (INSERT, UPDATE, DELETE), set `read_only = false` and use `annotations { mode = "write" }`:

```hcl
command "insert_order" {
  annotations {
    mode = "write"
  }

  operation {
    protocol = "sql"
    sql {
      connection_secret = "analytics.db_url"
      query  = "INSERT INTO orders (customer, total) VALUES (?, ?)"
      params = ["{{ args.customer }}", "{{ args.total }}"]
      sandbox {
        read_only = false
      }
    }
  }
}
```

For advanced auth, see [secrets-and-auth.md](secrets-and-auth.md).
