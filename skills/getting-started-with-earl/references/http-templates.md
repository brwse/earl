# HTTP Templates

Use HTTP when calling REST APIs, webhooks, or any HTTP endpoint.

## Template Skeleton

```hcl
version = 1
provider = "myapi"

command "get_items" {
  title       = "Get Items"
  summary     = "Fetch items from the API"
  description = "Retrieves a list of items with optional filtering"

  annotations {
    mode = "read"
    secrets = ["myapi.token"]
  }

  param "query" {
    type        = "string"
    required    = true
    description = "Search query"
  }

  param "limit" {
    type        = "integer"
    required    = false
    description = "Max results to return"
    default     = 10
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.example.com/items"

    query = {
      q     = "{{ args.query }}"
      limit = "{{ args.limit }}"
    }

    headers = {
      Accept = "application/json"
    }

    auth {
      kind   = "bearer"
      secret = "myapi.token"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} items"
  }
}
```

## Key Fields

| Field     | Required | Description                                    |
| --------- | -------- | ---------------------------------------------- |
| `method`  | Yes      | HTTP method: GET, POST, PUT, PATCH, DELETE     |
| `url`     | Yes      | Full URL (supports `{{ args.* }}` expressions) |
| `query`   | No       | Query parameters as key-value map              |
| `headers` | No       | HTTP headers as key-value map                  |
| `body`    | No       | Request body (see body kinds below)            |
| `auth`    | No       | Authentication block                           |

**Note:** HTTP operation fields are flat (directly in the `operation` block), unlike other protocols which use nested sub-blocks.

## Body Kinds

For POST/PUT/PATCH, add a `body` block inside `operation`:

```hcl
# JSON body
body {
  kind  = "json"
  value = {
    title = "{{ args.title }}"
    body  = "{{ args.body }}"
  }
}

# Form body
body {
  kind  = "form"
  value = {
    username = "{{ args.username }}"
    password = "{{ args.password }}"
  }
}

# Raw text body
body {
  kind  = "raw"
  value = "{{ args.content }}"
}
```

## Auth Patterns

```hcl
# Bearer token (most common for REST APIs)
auth {
  kind   = "bearer"
  secret = "myapi.token"
}

# Basic auth
auth {
  kind   = "basic"
  secret = "myapi.credentials"
}

# API key in header (use headers instead of auth block)
headers = {
  X-API-Key = "{{ secrets.myapi_key }}"
}
```

Set the secret: `earl secrets set myapi.token`

For advanced auth (OAuth, profiles), see [secrets-and-auth.md](secrets-and-auth.md).

## Common Patterns

**URL parameters:**

```hcl
url = "https://api.example.com/users/{{ args.user_id }}/repos"
```

**Result extraction with JSON pointer:**

```hcl
result {
  decode  = "json"
  extract = { json_pointer = "/items" }
  output  = "Found {{ result | length }} items"
}
```

**Write-mode command (requires user confirmation):**

```hcl
annotations {
  mode = "write"
}
```
