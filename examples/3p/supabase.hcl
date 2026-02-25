version = 1
provider = "supabase"
categories = ["database", "backend", "infrastructure"]

command "list_projects" {
  title       = "List projects"
  summary     = "List all Supabase projects"
  description = "List all projects accessible with the current access token."
  categories  = ["projects"]

  annotations {
    mode    = "read"
    secrets = ["supabase.access_token"]
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.supabase.com/v1/projects"

    auth {
      kind   = "bearer"
      secret = "supabase.access_token"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} projects."
  }
}

command "get_project" {
  title       = "Get project"
  summary     = "Get details of a Supabase project"
  description = "Retrieve detailed information about a specific Supabase project including region, status, and database configuration."
  categories  = ["projects"]

  annotations {
    mode    = "read"
    secrets = ["supabase.access_token"]
  }

  param "project_ref" {
    type        = "string"
    required    = true
    description = "Project reference ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.supabase.com/v1/projects/{{ args.project_ref }}"

    auth {
      kind   = "bearer"
      secret = "supabase.access_token"
    }
  }

  result {
    decode = "json"
    output = "Project: {{ result.name }} ({{ result.ref }}) [{{ result.status }}] — {{ result.region }}"
  }
}

command "create_project" {
  title       = "Create project"
  summary     = "Create a new Supabase project"
  description = "Create a new Supabase project in the specified organization with a database password and region."
  categories  = ["projects"]

  annotations {
    mode    = "write"
    secrets = ["supabase.access_token"]
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Project name"
  }

  param "organization_slug" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  param "db_pass" {
    type        = "string"
    required    = true
    description = "Database password"
  }

  param "region" {
    type        = "string"
    required    = false
    default     = "us-east-1"
    description = "AWS region (e.g. us-east-1, eu-west-1, ap-southeast-1)"
  }

  param "plan" {
    type        = "string"
    required    = false
    default     = "free"
    description = "Plan tier: free or pro"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.supabase.com/v1/projects"

    auth {
      kind   = "bearer"
      secret = "supabase.access_token"
    }

    body {
      kind = "json"
      value = {
        name              = "{{ args.name }}"
        organization_slug = "{{ args.organization_slug }}"
        db_pass           = "{{ args.db_pass }}"
        region            = "{{ args.region }}"
        plan              = "{{ args.plan }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created project: {{ result.name }} ({{ result.ref }}) [{{ result.region }}]"
  }
}

command "get_project_health" {
  title       = "Get project health"
  summary     = "Check health of project services"
  description = "Check the health status of auth, REST, database, and storage services for a Supabase project."
  categories  = ["projects"]

  annotations {
    mode    = "read"
    secrets = ["supabase.access_token"]
  }

  param "project_ref" {
    type        = "string"
    required    = true
    description = "Project reference ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.supabase.com/v1/projects/{{ args.project_ref }}/health?services=auth&services=rest&services=db&services=storage"

    auth {
      kind   = "bearer"
      secret = "supabase.access_token"
    }
  }

  result {
    decode = "json"
    output = "Health check returned {{ result | length }} service(s) for {{ args.project_ref }}."
  }
}

command "run_sql" {
  title       = "Run SQL query"
  summary     = "Execute a SQL query against a project database"
  description = "Run an arbitrary SQL query against the Postgres database of a Supabase project via the Management API."
  categories  = ["database"]

  annotations {
    mode    = "write"
    secrets = ["supabase.access_token"]
  }

  param "project_ref" {
    type        = "string"
    required    = true
    description = "Project reference ID"
  }

  param "query" {
    type        = "string"
    required    = true
    description = "SQL query to execute"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.supabase.com/v1/projects/{{ args.project_ref }}/database/query"

    auth {
      kind   = "bearer"
      secret = "supabase.access_token"
    }

    body {
      kind = "json"
      value = {
        query = "{{ args.query }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Query returned {{ result | length }} rows."
  }
}

command "list_secrets" {
  title       = "List secrets"
  summary     = "List all secrets for a project"
  description = "List all environment variable secrets configured for a Supabase project. Values are not returned, only names and metadata."
  categories  = ["secrets"]

  annotations {
    mode    = "read"
    secrets = ["supabase.access_token"]
  }

  param "project_ref" {
    type        = "string"
    required    = true
    description = "Project reference ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.supabase.com/v1/projects/{{ args.project_ref }}/secrets"

    auth {
      kind   = "bearer"
      secret = "supabase.access_token"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} secret(s) for {{ args.project_ref }}."
  }
}

command "list_edge_functions" {
  title       = "List edge functions"
  summary     = "List all edge functions for a project"
  description = "List all deployed edge functions for a Supabase project including name, slug, status, and version."
  categories  = ["functions"]

  annotations {
    mode    = "read"
    secrets = ["supabase.access_token"]
  }

  param "project_ref" {
    type        = "string"
    required    = true
    description = "Project reference ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.supabase.com/v1/projects/{{ args.project_ref }}/functions"

    auth {
      kind   = "bearer"
      secret = "supabase.access_token"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} edge function(s) for {{ args.project_ref }}."
  }
}

command "generate_types" {
  title       = "Generate TypeScript types"
  summary     = "Generate TypeScript types from database schema"
  description = "Generate TypeScript type definitions from the database schema of a Supabase project. Useful for type-safe client usage."
  categories  = ["development"]

  annotations {
    mode    = "read"
    secrets = ["supabase.access_token"]
  }

  param "project_ref" {
    type        = "string"
    required    = true
    description = "Project reference ID"
  }

  param "included_schemas" {
    type        = "string"
    required    = false
    default     = "public"
    description = "Comma-separated schema names to include"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.supabase.com/v1/projects/{{ args.project_ref }}/types/typescript"

    auth {
      kind   = "bearer"
      secret = "supabase.access_token"
    }

    query = {
      included_schemas = "{{ args.included_schemas }}"
    }
  }

  result {
    decode = "json"
    output = "{{ result.types }}"
  }
}

command "list_users" {
  title       = "List users"
  summary     = "List auth users for a project"
  description = "List all authentication users for a Supabase project via the Auth Admin API. Requires the project's service role key."
  categories  = ["auth"]

  annotations {
    mode    = "read"
    secrets = ["supabase.service_role_key"]
  }

  param "project_ref" {
    type        = "string"
    required    = true
    description = "Project reference ID"
  }

  param "page" {
    type        = "integer"
    required    = false
    default     = 1
    description = "Page number"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Users per page"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.project_ref }}.supabase.co/auth/v1/admin/users"

    auth {
      kind   = "bearer"
      secret = "supabase.service_role_key"
    }

    query = {
      page     = "{{ args.page }}"
      per_page = "{{ args.per_page }}"
    }

    headers = {
      apikey = "{{ secrets.supabase_service_role_key }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.users | length }} user(s)."
  }
}

command "create_user" {
  title       = "Create user"
  summary     = "Create a new auth user"
  description = "Create a new authentication user for a Supabase project via the Auth Admin API. The user is auto-confirmed by default."
  categories  = ["auth"]

  annotations {
    mode    = "write"
    secrets = ["supabase.service_role_key"]
  }

  param "project_ref" {
    type        = "string"
    required    = true
    description = "Project reference ID"
  }

  param "email" {
    type        = "string"
    required    = true
    description = "User email address"
  }

  param "email_confirm" {
    type        = "boolean"
    required    = false
    default     = true
    description = "Auto-confirm the email address"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ args.project_ref }}.supabase.co/auth/v1/admin/users"

    auth {
      kind   = "bearer"
      secret = "supabase.service_role_key"
    }

    headers = {
      apikey = "{{ secrets.supabase_service_role_key }}"
    }

    body {
      kind = "json"
      value = {
        email         = "{{ args.email }}"
        email_confirm = "{{ args.email_confirm }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created user: {{ result.email }} ({{ result.id }})"
  }
}

command "delete_user" {
  title       = "Delete user"
  summary     = "Delete an auth user"
  description = "Permanently delete an authentication user from a Supabase project by their UUID."
  categories  = ["auth"]

  annotations {
    mode    = "write"
    secrets = ["supabase.service_role_key"]
  }

  param "project_ref" {
    type        = "string"
    required    = true
    description = "Project reference ID"
  }

  param "user_id" {
    type        = "string"
    required    = true
    description = "User UUID"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://{{ args.project_ref }}.supabase.co/auth/v1/admin/users/{{ args.user_id }}"

    auth {
      kind   = "bearer"
      secret = "supabase.service_role_key"
    }

    headers = {
      apikey = "{{ secrets.supabase_service_role_key }}"
    }
  }

  result {
    decode = "json"
    output = "Deleted user {{ args.user_id }}."
  }
}

command "query_table" {
  title       = "Query table"
  summary     = "Query rows from a table via PostgREST"
  description = "Query rows from a table or view using the PostgREST API. Supports column selection and row limits. For filtering, append PostgREST operators as query params (e.g. status=eq.active)."
  categories  = ["database"]

  annotations {
    mode    = "read"
    secrets = ["supabase.service_role_key"]
  }

  param "project_ref" {
    type        = "string"
    required    = true
    description = "Project reference ID"
  }

  param "table" {
    type        = "string"
    required    = true
    description = "Table or view name"
  }

  param "select" {
    type        = "string"
    required    = false
    default     = "*"
    description = "Column selection (e.g. id,name,email)"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Maximum number of rows to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.project_ref }}.supabase.co/rest/v1/{{ args.table }}"

    auth {
      kind   = "bearer"
      secret = "supabase.service_role_key"
    }

    query = {
      select = "{{ args.select }}"
      limit  = "{{ args.limit }}"
    }

    headers = {
      apikey = "{{ secrets.supabase_service_role_key }}"
    }
  }

  result {
    decode = "json"
    output = "Returned {{ result | length }} rows from {{ args.table }}."
  }
}

command "invoke_function" {
  title       = "Invoke edge function"
  summary     = "Invoke a deployed edge function"
  description = "Invoke a deployed Supabase edge function by its slug. Sends an empty JSON body by default — modify the template to pass custom payloads."
  categories  = ["functions"]

  annotations {
    mode    = "write"
    secrets = ["supabase.service_role_key"]
  }

  param "project_ref" {
    type        = "string"
    required    = true
    description = "Project reference ID"
  }

  param "function_name" {
    type        = "string"
    required    = true
    description = "Edge function slug"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ args.project_ref }}.supabase.co/functions/v1/{{ args.function_name }}"

    auth {
      kind   = "bearer"
      secret = "supabase.service_role_key"
    }

    body {
      kind = "json"
      value = {}
    }
  }

  result {
    decode = "json"
    output = "Function {{ args.function_name }} invoked successfully."
  }
}
