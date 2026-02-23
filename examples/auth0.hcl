version = 1
provider = "auth0"
categories = ["identity", "authentication", "security"]

command "list_users" {
  title       = "List users"
  summary     = "List or search users in your Auth0 tenant"
  description = "Retrieve users from the Auth0 Management API with optional Lucene query filtering and pagination. Requires the AUTH0_DOMAIN environment variable to be set to your tenant domain (e.g. mytenant.us.auth0.com)."
  categories  = ["users"]

  annotations {
    mode    = "read"
    secrets = ["auth0.token"]
  }

  param "q" {
    type        = "string"
    required    = false
    default     = ""
    description = "Lucene query string (e.g. 'email:\"jane@example.com\"')"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Results per page (max 100)"
  }

  param "page" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Zero-based page number"
  }

  param "sort" {
    type        = "string"
    required    = false
    default     = ""
    description = "Sort field and direction (e.g. 'created_at:-1')"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/users"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    query = {
      q              = "{{ args.q }}"
      search_engine  = "v3"
      per_page       = "{{ args.per_page }}"
      page           = "{{ args.page }}"
      sort           = "{{ args.sort }}"
      include_totals = "true"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Found {{ result.total }} users (page {{ result.start }}):
    {% for user in result.users %}
    - {{ user.email }} ({{ user.user_id }}) — {{ user.connection }} — verified: {{ user.email_verified }} — logins: {{ user.logins_count }}
    {% endfor %}
    EOF
  }
}

command "get_user" {
  title       = "Get user"
  summary     = "Get a user by ID"
  description = "Retrieve detailed information about a specific Auth0 user by their user_id."
  categories  = ["users"]

  annotations {
    mode    = "read"
    secrets = ["auth0.token"]
  }

  param "user_id" {
    type        = "string"
    required    = true
    description = "User ID (e.g. 'auth0|507f1f77bcf86cd799439020')"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/users/{{ args.user_id }}"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    User: {{ result.name }} ({{ result.user_id }})
    Email: {{ result.email }} (verified: {{ result.email_verified }})
    Connection: {{ result.identities[0].connection }}
    Created: {{ result.created_at }}
    Last login: {{ result.last_login }}
    Login count: {{ result.logins_count }}
    Blocked: {{ result.blocked | default("false") }}
    EOF
  }
}

command "create_user" {
  title       = "Create user"
  summary     = "Create a new user"
  description = "Create a new user in the specified Auth0 connection with email and password."
  categories  = ["users"]

  annotations {
    mode    = "write"
    secrets = ["auth0.token"]
  }

  param "connection" {
    type        = "string"
    required    = true
    description = "Connection name (e.g. 'Username-Password-Authentication')"
  }

  param "email" {
    type        = "string"
    required    = true
    description = "User's email address"
  }

  param "password" {
    type        = "string"
    required    = true
    description = "User's password"
  }

  param "name" {
    type        = "string"
    required    = false
    default     = ""
    description = "User's full name"
  }

  param "email_verified" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Mark email as pre-verified"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/users"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        connection     = "{{ args.connection }}"
        email          = "{{ args.email }}"
        password       = "{{ args.password }}"
        name           = "{{ args.name }}"
        email_verified = "{{ args.email_verified }}"
      }
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Created user: {{ result.email }} ({{ result.user_id }})
    Connection: {{ result.identities[0].connection }}
    Email verified: {{ result.email_verified }}
    EOF
  }
}

command "update_user" {
  title       = "Update user"
  summary     = "Update user attributes"
  description = "Update properties of an existing Auth0 user such as name, email, blocked status, or metadata."
  categories  = ["users"]

  annotations {
    mode    = "write"
    secrets = ["auth0.token"]
  }

  param "user_id" {
    type        = "string"
    required    = true
    description = "The user_id to update"
  }

  param "name" {
    type        = "string"
    required    = false
    description = "Updated full name"
  }

  param "email" {
    type        = "string"
    required    = false
    description = "Updated email address"
  }

  param "blocked" {
    type        = "boolean"
    required    = false
    description = "Block or unblock the user"
  }

  param "email_verified" {
    type        = "boolean"
    required    = false
    description = "Mark email as verified or unverified"
  }

  param "connection" {
    type        = "string"
    required    = false
    description = "Required when updating email or phone"
  }

  param "client_id" {
    type        = "string"
    required    = false
    description = "Required when updating email or phone"
  }

  operation {
    protocol = "http"
    method   = "PATCH"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/users/{{ args.user_id }}"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        name           = "{{ args.name }}"
        email          = "{{ args.email }}"
        blocked        = "{{ args.blocked }}"
        email_verified = "{{ args.email_verified }}"
        connection     = "{{ args.connection }}"
        client_id      = "{{ args.client_id }}"
      }
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Updated user: {{ result.email }} ({{ result.user_id }})
    Name: {{ result.name }}
    Blocked: {{ result.blocked | default("false") }}
    Updated at: {{ result.updated_at }}
    EOF
  }
}

command "delete_user" {
  title       = "Delete user"
  summary     = "Delete a user by ID"
  description = "Permanently delete a user from Auth0. This action cannot be undone."
  categories  = ["users"]

  annotations {
    mode    = "write"
    secrets = ["auth0.token"]
  }

  param "user_id" {
    type        = "string"
    required    = true
    description = "The user_id to delete"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/users/{{ args.user_id }}"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Deleted user: {{ args.user_id }}"
  }
}

command "list_roles" {
  title       = "List roles"
  summary     = "List all roles defined in the tenant"
  description = "Retrieve all roles with optional name filter and pagination."
  categories  = ["roles"]

  annotations {
    mode    = "read"
    secrets = ["auth0.token"]
  }

  param "name_filter" {
    type        = "string"
    required    = false
    default     = ""
    description = "Case-insensitive filter by role name"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Results per page (max 100)"
  }

  param "page" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Zero-based page number"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/roles"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    query = {
      name_filter    = "{{ args.name_filter }}"
      per_page       = "{{ args.per_page }}"
      page           = "{{ args.page }}"
      include_totals = "true"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Found {{ result.total }} roles:
    {% for role in result.roles %}
    - {{ role.name }} ({{ role.id }}): {{ role.description | default("no description") }}
    {% endfor %}
    EOF
  }
}

command "assign_user_roles" {
  title       = "Assign roles to user"
  summary     = "Assign one or more roles to a user"
  description = "Assign roles to a user by providing the user ID and an array of role IDs."
  categories  = ["users", "roles"]

  annotations {
    mode    = "write"
    secrets = ["auth0.token"]
  }

  param "user_id" {
    type        = "string"
    required    = true
    description = "The user_id to assign roles to"
  }

  param "roles" {
    type        = "array"
    required    = true
    description = "Array of role IDs to assign"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/users/{{ args.user_id }}/roles"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        roles = "{{ args.roles }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Assigned {{ args.roles | length }} role(s) to user {{ args.user_id }}."
  }
}

command "list_clients" {
  title       = "List applications"
  summary     = "List all applications (clients) in the tenant"
  description = "Retrieve all registered applications with optional type filtering and pagination."
  categories  = ["applications"]

  annotations {
    mode    = "read"
    secrets = ["auth0.token"]
  }

  param "app_type" {
    type        = "string"
    required    = false
    default     = ""
    description = "Filter by type: native, spa, regular_web, or non_interactive"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Results per page (max 100)"
  }

  param "page" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Zero-based page number"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/clients"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    query = {
      app_type       = "{{ args.app_type }}"
      per_page       = "{{ args.per_page }}"
      page           = "{{ args.page }}"
      include_totals = "true"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Found {{ result.total }} applications:
    {% for client in result.clients %}
    - {{ client.name }} ({{ client.client_id }}) — type: {{ client.app_type | default("unknown") }} — callbacks: {{ client.callbacks | default([]) | length }}
    {% endfor %}
    EOF
  }
}

command "get_client" {
  title       = "Get application"
  summary     = "Get application details by client ID"
  description = "Retrieve detailed information about a specific Auth0 application."
  categories  = ["applications"]

  annotations {
    mode    = "read"
    secrets = ["auth0.token"]
  }

  param "client_id" {
    type        = "string"
    required    = true
    description = "The application's client_id"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/clients/{{ args.client_id }}"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Application: {{ result.name }} ({{ result.client_id }})
    Type: {{ result.app_type | default("unknown") }}
    Callbacks: {{ result.callbacks | default([]) | join(", ") }}
    Allowed Origins: {{ result.allowed_origins | default([]) | join(", ") }}
    Grant Types: {{ result.grant_types | default([]) | join(", ") }}
    EOF
  }
}

command "create_client" {
  title       = "Create application"
  summary     = "Register a new application"
  description = "Create a new application (client) in Auth0 with the specified name and type."
  categories  = ["applications"]

  annotations {
    mode    = "write"
    secrets = ["auth0.token"]
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Application name"
  }

  param "app_type" {
    type        = "string"
    required    = false
    default     = "regular_web"
    description = "Application type: native, spa, regular_web, or non_interactive"
  }

  param "description" {
    type        = "string"
    required    = false
    default     = ""
    description = "Application description"
  }

  param "callbacks" {
    type        = "array"
    required    = false
    description = "Allowed callback URLs"
  }

  param "allowed_origins" {
    type        = "array"
    required    = false
    description = "Allowed origins for CORS"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/clients"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        name            = "{{ args.name }}"
        app_type        = "{{ args.app_type }}"
        description     = "{{ args.description }}"
        callbacks       = "{{ args.callbacks }}"
        allowed_origins = "{{ args.allowed_origins }}"
      }
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Created application: {{ result.name }} ({{ result.client_id }})
    Type: {{ result.app_type | default("unknown") }}
    Client Secret: {{ result.client_secret }}
    EOF
  }
}

command "list_connections" {
  title       = "List connections"
  summary     = "List identity provider connections"
  description = "Retrieve all identity provider connections configured in the tenant, optionally filtered by strategy."
  categories  = ["connections"]

  annotations {
    mode    = "read"
    secrets = ["auth0.token"]
  }

  param "strategy" {
    type        = "string"
    required    = false
    default     = ""
    description = "Filter by strategy (e.g. 'auth0', 'google-oauth2', 'samlp')"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Results per page (max 100)"
  }

  param "page" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Zero-based page number"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/connections"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    query = {
      strategy       = "{{ args.strategy }}"
      per_page       = "{{ args.per_page }}"
      page           = "{{ args.page }}"
      include_totals = "true"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Found {{ result.total }} connections:
    {% for conn in result.connections %}
    - {{ conn.name }} ({{ conn.id }}) — strategy: {{ conn.strategy }} — enabled clients: {{ conn.enabled_clients | length }}
    {% endfor %}
    EOF
  }
}

command "list_organizations" {
  title       = "List organizations"
  summary     = "List all organizations in the tenant"
  description = "Retrieve organizations with pagination support."
  categories  = ["organizations"]

  annotations {
    mode    = "read"
    secrets = ["auth0.token"]
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Results per page (max 100)"
  }

  param "page" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Zero-based page number"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/organizations"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    query = {
      per_page       = "{{ args.per_page }}"
      page           = "{{ args.page }}"
      include_totals = "true"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Found {{ result.total }} organizations:
    {% for org in result.organizations %}
    - {{ org.display_name | default(org.name) }} ({{ org.id }}) — name: {{ org.name }}
    {% endfor %}
    EOF
  }
}

command "create_organization" {
  title       = "Create organization"
  summary     = "Create a new organization"
  description = "Create a new organization for multi-tenant B2B use cases."
  categories  = ["organizations"]

  annotations {
    mode    = "write"
    secrets = ["auth0.token"]
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Organization name (lowercase, alphanumeric and hyphens only)"
  }

  param "display_name" {
    type        = "string"
    required    = false
    default     = ""
    description = "Human-readable display name"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/organizations"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        name         = "{{ args.name }}"
        display_name = "{{ args.display_name }}"
      }
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Created organization: {{ result.display_name | default(result.name) }} ({{ result.id }})
    Name: {{ result.name }}
    EOF
  }
}

command "list_logs" {
  title       = "List log events"
  summary     = "Search tenant log events"
  description = "Retrieve log events from the Auth0 tenant with optional Lucene query filtering. Common event types: s (success login), f (failed login), ss (signup), du (deleted user)."
  categories  = ["logs"]

  annotations {
    mode    = "read"
    secrets = ["auth0.token"]
  }

  param "q" {
    type        = "string"
    required    = false
    default     = ""
    description = "Lucene query string to filter events"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Results per page (max 100)"
  }

  param "page" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Zero-based page number"
  }

  param "sort" {
    type        = "string"
    required    = false
    default     = "date:-1"
    description = "Sort field and direction (e.g. 'date:-1' for newest first)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/logs"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    query = {
      q              = "{{ args.q }}"
      per_page       = "{{ args.per_page }}"
      page           = "{{ args.page }}"
      sort           = "{{ args.sort }}"
      include_totals = "true"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Found {{ result.total }} log events:
    {% for log in result.logs %}
    - [{{ log.date }}] {{ log.type }} — {{ log.description | default("") }} — user: {{ log.user_name | default("n/a") }} — IP: {{ log.ip | default("n/a") }}
    {% endfor %}
    EOF
  }
}

command "get_log" {
  title       = "Get log event"
  summary     = "Get a single log event by ID"
  description = "Retrieve detailed information about a specific log event."
  categories  = ["logs"]

  annotations {
    mode    = "read"
    secrets = ["auth0.token"]
  }

  param "log_id" {
    type        = "string"
    required    = true
    description = "The log event ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.AUTH0_DOMAIN }}/api/v2/logs/{{ args.log_id }}"

    auth {
      kind   = "bearer"
      secret = "auth0.token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Log Event: {{ result._id }}
    Type: {{ result.type }} — {{ result.description | default("") }}
    Date: {{ result.date }}
    User: {{ result.user_name | default("n/a") }} ({{ result.user_id | default("n/a") }})
    IP: {{ result.ip | default("n/a") }}
    Client: {{ result.client_name | default("n/a") }} ({{ result.client_id | default("n/a") }})
    Connection: {{ result.connection | default("n/a") }}
    EOF
  }
}
