version = 1
provider = "render"
categories = ["cloud", "hosting", "infrastructure"]

command "list_services" {
  title       = "List services"
  summary     = "List all services in your Render account"
  description = "List services with optional filters for name, type, region, and suspension status."
  categories  = ["services"]

  annotations {
    mode    = "read"
    secrets = ["render.api_key"]
  }

  param "name" {
    type        = "string"
    required    = false
    description = "Filter by service name"
  }

  param "type" {
    type        = "string"
    required    = false
    description = "Filter by type (web_service, static_site, private_service, background_worker, cron_job)"
  }

  param "region" {
    type        = "string"
    required    = false
    description = "Filter by region (oregon, frankfurt, singapore, ohio, virginia)"
  }

  param "suspended" {
    type        = "string"
    required    = false
    description = "Filter by suspension status (suspended, not_suspended)"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Max results to return (1-100)"
  }

  param "cursor" {
    type        = "string"
    required    = false
    description = "Pagination cursor from a previous response"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.render.com/v1/services"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    query = {
      name     = "{{ args.name }}"
      type     = "{{ args.type }}"
      region   = "{{ args.region }}"
      suspended = "{{ args.suspended }}"
      limit    = "{{ args.limit }}"
      cursor   = "{{ args.cursor }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} service(s):\n{% for item in result %}\n- {{ item.service.name }} ({{ item.service.id }}) — {{ item.service.type }} / {{ item.service.serviceDetails.region }} — {{ item.service.suspended }}\n{% endfor %}"
  }
}

command "get_service" {
  title       = "Get service"
  summary     = "Get details of a specific service"
  description = "Retrieve full details for a Render service by its ID."
  categories  = ["services"]

  annotations {
    mode    = "read"
    secrets = ["render.api_key"]
  }

  param "service_id" {
    type        = "string"
    required    = true
    description = "Service ID (e.g. srv-...)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.render.com/v1/services/{{ args.service_id }}"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Service: {{ result.name }} ({{ result.id }})\nType: {{ result.type }}\nRegion: {{ result.serviceDetails.region }}\nURL: {{ result.serviceDetails.url }}\nSuspended: {{ result.suspended }}\nAuto-deploy: {{ result.autoDeploy }}\nRepo: {{ result.repo }}\nBranch: {{ result.branch }}\nCreated: {{ result.createdAt }}\nUpdated: {{ result.updatedAt }}"
  }
}

command "list_deploys" {
  title       = "List deploys"
  summary     = "List deploys for a service"
  description = "List recent deploys for a Render service, showing status and commit info."
  categories  = ["deploys"]

  annotations {
    mode    = "read"
    secrets = ["render.api_key"]
  }

  param "service_id" {
    type        = "string"
    required    = true
    description = "Service ID"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Max results to return (1-100)"
  }

  param "cursor" {
    type        = "string"
    required    = false
    description = "Pagination cursor from a previous response"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.render.com/v1/services/{{ args.service_id }}/deploys"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    query = {
      limit  = "{{ args.limit }}"
      cursor = "{{ args.cursor }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Deploys for {{ args.service_id }}:\n{% for item in result %}\n- {{ item.deploy.id }} — {{ item.deploy.status }} — commit {{ item.deploy.commit.id[:7] }} — {{ item.deploy.createdAt }}\n{% endfor %}"
  }
}

command "get_deploy" {
  title       = "Get deploy"
  summary     = "Get details of a specific deploy"
  description = "Retrieve full details for a deploy by service and deploy ID."
  categories  = ["deploys"]

  annotations {
    mode    = "read"
    secrets = ["render.api_key"]
  }

  param "service_id" {
    type        = "string"
    required    = true
    description = "Service ID"
  }

  param "deploy_id" {
    type        = "string"
    required    = true
    description = "Deploy ID (e.g. dep-...)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.render.com/v1/services/{{ args.service_id }}/deploys/{{ args.deploy_id }}"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Deploy: {{ result.id }}\nStatus: {{ result.status }}\nCommit: {{ result.commit.id }} — {{ result.commit.message }}\nCreated: {{ result.createdAt }}\nFinished: {{ result.finishedAt }}"
  }
}

command "trigger_deploy" {
  title       = "Trigger deploy"
  summary     = "Trigger a new deploy for a service"
  description = "Trigger a manual deploy for a Render service, optionally clearing the build cache or deploying a specific commit."
  categories  = ["deploys"]

  annotations {
    mode    = "write"
    secrets = ["render.api_key"]
  }

  param "service_id" {
    type        = "string"
    required    = true
    description = "Service ID"
  }

  param "clear_cache" {
    type        = "string"
    required    = false
    default     = "do_not_clear"
    description = "Whether to clear build cache (clear or do_not_clear)"
  }

  param "commit_id" {
    type        = "string"
    required    = false
    description = "Specific git commit SHA to deploy"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.render.com/v1/services/{{ args.service_id }}/deploys"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        clearCache = "{{ args.clear_cache }}"
        commitId   = "{{ args.commit_id }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Deploy triggered: {{ result.id }}\nStatus: {{ result.status }}"
  }
}

command "rollback_deploy" {
  title       = "Rollback deploy"
  summary     = "Roll back a service to a previous deploy"
  description = "Initiate a rollback of a service to a specific previous deploy."
  categories  = ["deploys"]

  annotations {
    mode    = "write"
    secrets = ["render.api_key"]
  }

  param "service_id" {
    type        = "string"
    required    = true
    description = "Service ID"
  }

  param "deploy_id" {
    type        = "string"
    required    = true
    description = "Deploy ID to roll back to"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.render.com/v1/services/{{ args.service_id }}/rollback"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        deployId = "{{ args.deploy_id }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Rollback initiated: {{ result.id }}\nStatus: {{ result.status }}\nRolling back to deploy: {{ args.deploy_id }}"
  }
}

command "list_env_vars" {
  title       = "List environment variables"
  summary     = "List environment variables for a service"
  description = "Retrieve all environment variables configured on a Render service."
  categories  = ["env"]

  annotations {
    mode    = "read"
    secrets = ["render.api_key"]
  }

  param "service_id" {
    type        = "string"
    required    = true
    description = "Service ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.render.com/v1/services/{{ args.service_id }}/env-vars"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Environment variables for {{ args.service_id }}:\n{% for item in result %}\n- {{ item.envVar.key }} = {{ item.envVar.value }}\n{% endfor %}"
  }
}

command "set_env_var" {
  title       = "Set environment variable"
  summary     = "Set an environment variable on a service"
  description = "Create or update an environment variable on a Render service."
  categories  = ["env"]

  annotations {
    mode    = "write"
    secrets = ["render.api_key"]
  }

  param "service_id" {
    type        = "string"
    required    = true
    description = "Service ID"
  }

  param "key" {
    type        = "string"
    required    = true
    description = "Environment variable name"
  }

  param "value" {
    type        = "string"
    required    = true
    description = "Environment variable value"
  }

  operation {
    protocol = "http"
    method   = "PUT"
    url      = "https://api.render.com/v1/services/{{ args.service_id }}/env-vars/{{ args.key }}"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        value = "{{ args.value }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Set {{ result.envVar.key }} = {{ result.envVar.value }} on service {{ args.service_id }}"
  }
}

command "restart_service" {
  title       = "Restart service"
  summary     = "Restart a running service"
  description = "Restart a Render service without triggering a new build."
  categories  = ["services"]

  annotations {
    mode    = "write"
    secrets = ["render.api_key"]
  }

  param "service_id" {
    type        = "string"
    required    = true
    description = "Service ID"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.render.com/v1/services/{{ args.service_id }}/restart"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Service {{ args.service_id }} restart initiated."
  }
}

command "suspend_service" {
  title       = "Suspend service"
  summary     = "Suspend a running service"
  description = "Suspend a Render service, stopping it from running and incurring charges."
  categories  = ["services"]

  annotations {
    mode    = "write"
    secrets = ["render.api_key"]
  }

  param "service_id" {
    type        = "string"
    required    = true
    description = "Service ID"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.render.com/v1/services/{{ args.service_id }}/suspend"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Service {{ args.service_id }} suspended."
  }
}

command "resume_service" {
  title       = "Resume service"
  summary     = "Resume a suspended service"
  description = "Resume a previously suspended Render service."
  categories  = ["services"]

  annotations {
    mode    = "write"
    secrets = ["render.api_key"]
  }

  param "service_id" {
    type        = "string"
    required    = true
    description = "Service ID"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.render.com/v1/services/{{ args.service_id }}/resume"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Service {{ args.service_id }} resumed."
  }
}

command "list_custom_domains" {
  title       = "List custom domains"
  summary     = "List custom domains for a service"
  description = "List all custom domains configured on a Render service with verification status."
  categories  = ["domains"]

  annotations {
    mode    = "read"
    secrets = ["render.api_key"]
  }

  param "service_id" {
    type        = "string"
    required    = true
    description = "Service ID"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Max results to return (1-100)"
  }

  param "cursor" {
    type        = "string"
    required    = false
    description = "Pagination cursor from a previous response"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.render.com/v1/services/{{ args.service_id }}/custom-domains"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    query = {
      limit  = "{{ args.limit }}"
      cursor = "{{ args.cursor }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Custom domains for {{ args.service_id }}:\n{% for item in result %}\n- {{ item.customDomain.name }} — {{ item.customDomain.domainType }} — {{ item.customDomain.verificationStatus }}\n{% endfor %}"
  }
}

command "list_postgres" {
  title       = "List Postgres instances"
  summary     = "List all Postgres databases in your account"
  description = "List Render-managed Postgres instances with their plan, region, and status."
  categories  = ["databases"]

  annotations {
    mode    = "read"
    secrets = ["render.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Max results to return (1-100)"
  }

  param "cursor" {
    type        = "string"
    required    = false
    description = "Pagination cursor from a previous response"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.render.com/v1/postgres"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    query = {
      limit  = "{{ args.limit }}"
      cursor = "{{ args.cursor }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Postgres instances:\n{% for item in result %}\n- {{ item.postgres.name }} ({{ item.postgres.id }}) — {{ item.postgres.plan }} / {{ item.postgres.region }} — {{ item.postgres.status }}\n{% endfor %}"
  }
}

command "get_postgres" {
  title       = "Get Postgres instance"
  summary     = "Get details of a Postgres database"
  description = "Retrieve full details for a Render-managed Postgres instance by its ID."
  categories  = ["databases"]

  annotations {
    mode    = "read"
    secrets = ["render.api_key"]
  }

  param "postgres_id" {
    type        = "string"
    required    = true
    description = "Postgres instance ID (e.g. dpg-...)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.render.com/v1/postgres/{{ args.postgres_id }}"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Postgres: {{ result.name }} ({{ result.id }})\nPlan: {{ result.plan }}\nRegion: {{ result.region }}\nStatus: {{ result.status }}\nVersion: {{ result.version }}\nCreated: {{ result.createdAt }}"
  }
}

command "get_postgres_connection_info" {
  title       = "Get Postgres connection info"
  summary     = "Get connection strings for a Postgres database"
  description = "Retrieve connection details including internal and external connection strings for a Render Postgres instance."
  categories  = ["databases"]

  annotations {
    mode    = "read"
    secrets = ["render.api_key"]
  }

  param "postgres_id" {
    type        = "string"
    required    = true
    description = "Postgres instance ID (e.g. dpg-...)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.render.com/v1/postgres/{{ args.postgres_id }}/connection-info"

    auth {
      kind   = "bearer"
      secret = "render.api_key"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Connection info for {{ args.postgres_id }}:\nInternal URL: {{ result.internalConnectionString }}\nExternal URL: {{ result.externalConnectionString }}\nPSQL Command: {{ result.psqlCommand }}\nHost: {{ result.host }}\nPort: {{ result.port }}\nDatabase: {{ result.databaseName }}\nUser: {{ result.databaseUser }}"
  }
}
