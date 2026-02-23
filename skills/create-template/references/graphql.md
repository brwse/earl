# GraphQL Templates

Use GraphQL when calling a GraphQL API endpoint.

## Template Skeleton

```hcl
version = 1
provider = "myapi_graphql"

command "get_user" {
  title       = "Get User"
  summary     = "Fetch user profile from GraphQL API"
  description = <<-EOT
    Queries the authenticated user's profile from the GraphQL API.

    ## Guidance for AI agents

    Use this command to look up a user's profile by username.

    Example: `earl call myapi_graphql.get_user --username "alice"`
  EOT

  annotations {
    mode    = "read"
    secrets = ["myapi.token"]
  }

  param "username" {
    type        = "string"
    required    = true
    description = "Username to look up"
  }

  operation {
    protocol = "graphql"
    url      = "https://api.example.com/graphql"

    graphql {
      query = <<-EOT
        query GetUser($username: String!) {
          user(login: $username) {
            name
            email
            createdAt
          }
        }
      EOT

      variables = {
        username = "{{ args.username }}"
      }
    }

    auth {
      kind   = "bearer"
      secret = "myapi.token"
    }
  }

  result {
    decode  = "json"
    extract = { json_pointer = "/data/user" }
    output  = "User: {{ result.name }} ({{ result.email }})"
  }
}
```

## Key Fields

| Field               | Required | Description                                          |
| ------------------- | -------- | ---------------------------------------------------- |
| `url`               | Yes      | GraphQL endpoint URL                                 |
| `graphql.query`     | Yes      | The GraphQL query or mutation string                 |
| `graphql.variables` | No       | Variables as key-value map (supports `{{ args.* }}`) |
| `auth`              | No       | Authentication block (same as HTTP)                  |

**Note:** GraphQL uses a nested `graphql` block inside `operation`, unlike HTTP which is flat.

## Mutations

For write operations, use `annotations { mode = "write" }`:

```hcl
command "create_item" {
  annotations {
    mode = "write"
  }

  operation {
    protocol = "graphql"
    url      = "https://api.example.com/graphql"

    graphql {
      query = <<-EOT
        mutation CreateItem($title: String!) {
          createItem(input: { title: $title }) {
            id
            title
          }
        }
      EOT

      variables = {
        title = "{{ args.title }}"
      }
    }
  }
}
```

## Auth

GraphQL APIs typically use bearer tokens:

```hcl
auth {
  kind   = "bearer"
  secret = "myapi.token"
}
```

Set the secret: `earl secrets set myapi.token`

For advanced auth, see [secrets-and-auth.md](secrets-and-auth.md).
