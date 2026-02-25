version = 1
provider = "vercel"
categories = ["hosting", "deployment", "infrastructure"]

command "get_user" {
  title       = "Get user"
  summary     = "Get the authenticated user's profile"
  description = "Retrieve profile information for the currently authenticated Vercel user."
  categories  = ["read"]

  annotations {
    mode    = "read"
    secrets = ["vercel.token"]
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.vercel.com/v2/user"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }
  }

  result {
    decode = "json"
    output = "User: {{ result.user.username }} ({{ result.user.uid }}), Email: {{ result.user.email }}, Name: {{ result.user.name }}"
  }
}

command "list_projects" {
  title       = "List projects"
  summary     = "List Vercel projects"
  description = "Retrieve a list of projects, optionally filtered by name or repository."
  categories  = ["read", "projects"]

  annotations {
    mode    = "read"
    secrets = ["vercel.token"]
  }

  param "search" {
    type        = "string"
    required    = false
    description = "Search projects by name"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Maximum number of projects to return"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.vercel.com/v10/projects"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      search = "{{ args.search }}"
      limit  = "{{ args.limit }}"
      teamId = "{{ args.teamId }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.projects | length }} projects."
  }
}

command "get_project" {
  title       = "Get project"
  summary     = "Get details of a Vercel project"
  description = "Retrieve detailed information about a specific project by ID or name."
  categories  = ["read", "projects"]

  annotations {
    mode    = "read"
    secrets = ["vercel.token"]
  }

  param "id" {
    type        = "string"
    required    = true
    description = "Project ID or name"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.vercel.com/v10/projects/{{ args.id }}"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      teamId = "{{ args.teamId }}"
    }
  }

  result {
    decode = "json"
    output = "Project: {{ result.name }} ({{ result.id }}), Framework: {{ result.framework }}, Node: {{ result.nodeVersion }}, Updated: {{ result.updatedAt }}"
  }
}

command "create_project" {
  title       = "Create project"
  summary     = "Create a new Vercel project"
  description = "Create a new project in Vercel with the specified name and optional configuration."
  categories  = ["write", "projects"]

  annotations {
    mode    = "write"
    secrets = ["vercel.token"]
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Project name"
  }

  param "framework" {
    type        = "string"
    required    = false
    description = "Framework preset (e.g. nextjs, vite, remix)"
  }

  param "buildCommand" {
    type        = "string"
    required    = false
    description = "Custom build command"
  }

  param "outputDirectory" {
    type        = "string"
    required    = false
    description = "Custom output directory"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.vercel.com/v11/projects"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      teamId = "{{ args.teamId }}"
    }

    body {
      kind = "json"
      value = {
        name            = "{{ args.name }}"
        framework       = "{{ args.framework }}"
        buildCommand    = "{{ args.buildCommand }}"
        outputDirectory = "{{ args.outputDirectory }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Project created: {{ result.name }} ({{ result.id }}), Framework: {{ result.framework }}"
  }
}

command "delete_project" {
  title       = "Delete project"
  summary     = "Delete a Vercel project"
  description = "Permanently delete a project by ID or name. This action cannot be undone."
  categories  = ["write", "projects"]

  annotations {
    mode    = "write"
    secrets = ["vercel.token"]
  }

  param "id" {
    type        = "string"
    required    = true
    description = "Project ID or name"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.vercel.com/v10/projects/{{ args.id }}"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      teamId = "{{ args.teamId }}"
    }
  }

  result {
    decode = "json"
    output = "Project \"{{ args.id }}\" deleted."
  }
}

command "list_deployments" {
  title       = "List deployments"
  summary     = "List Vercel deployments"
  description = "Retrieve a list of deployments, optionally filtered by project, state, or target environment."
  categories  = ["read", "deployments"]

  annotations {
    mode    = "read"
    secrets = ["vercel.token"]
  }

  param "projectId" {
    type        = "string"
    required    = false
    description = "Filter by project ID or name"
  }

  param "state" {
    type        = "string"
    required    = false
    description = "Filter by state: BUILDING, ERROR, INITIALIZING, QUEUED, READY, CANCELED"
  }

  param "target" {
    type        = "string"
    required    = false
    description = "Filter by environment (production or preview)"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Maximum number of deployments to return"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.vercel.com/v6/deployments"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      projectId = "{{ args.projectId }}"
      state     = "{{ args.state }}"
      target    = "{{ args.target }}"
      limit     = "{{ args.limit }}"
      teamId    = "{{ args.teamId }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.deployments | length }} deployments."
  }
}

command "get_deployment" {
  title       = "Get deployment"
  summary     = "Get details of a specific deployment"
  description = "Retrieve detailed information about a deployment by its ID or hostname."
  categories  = ["read", "deployments"]

  annotations {
    mode    = "read"
    secrets = ["vercel.token"]
  }

  param "id" {
    type        = "string"
    required    = true
    description = "Deployment ID or hostname"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.vercel.com/v13/deployments/{{ args.id }}"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      teamId = "{{ args.teamId }}"
    }
  }

  result {
    decode = "json"
    output = "Deployment {{ result.id }}: {{ result.name }}, URL: {{ result.url }}, Status: {{ result.readyState }}, Target: {{ result.target }}, Created: {{ result.createdAt }}"
  }
}

command "create_deployment" {
  title       = "Create deployment"
  summary     = "Create a new Vercel deployment"
  description = "Trigger a new deployment for a project. Can optionally target a specific environment or redeploy an existing deployment."
  categories  = ["write", "deployments"]

  annotations {
    mode    = "write"
    secrets = ["vercel.token"]
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Project name used in the deployment URL"
  }

  param "project" {
    type        = "string"
    required    = false
    description = "Target project identifier"
  }

  param "target" {
    type        = "string"
    required    = false
    description = "Target environment: staging, production, or a custom environment ID"
  }

  param "deploymentId" {
    type        = "string"
    required    = false
    description = "Existing deployment ID to redeploy"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.vercel.com/v13/deployments"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      teamId = "{{ args.teamId }}"
    }

    body {
      kind = "json"
      value = {
        name         = "{{ args.name }}"
        project      = "{{ args.project }}"
        target       = "{{ args.target }}"
        deploymentId = "{{ args.deploymentId }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Deployment created: {{ result.id }}, URL: {{ result.url }}, Status: {{ result.readyState }}, Project: {{ result.name }}"
  }
}

command "cancel_deployment" {
  title       = "Cancel deployment"
  summary     = "Cancel a running deployment"
  description = "Cancel a deployment that is currently building or queued."
  categories  = ["write", "deployments"]

  annotations {
    mode    = "write"
    secrets = ["vercel.token"]
  }

  param "id" {
    type        = "string"
    required    = true
    description = "Deployment ID to cancel"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "PATCH"
    url      = "https://api.vercel.com/v12/deployments/{{ args.id }}/cancel"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      teamId = "{{ args.teamId }}"
    }
  }

  result {
    decode = "json"
    output = "Deployment {{ result.id }} canceled. State: {{ result.readyState }}"
  }
}

command "delete_deployment" {
  title       = "Delete deployment"
  summary     = "Delete a Vercel deployment"
  description = "Permanently delete a deployment by ID. This action cannot be undone."
  categories  = ["write", "deployments"]

  annotations {
    mode    = "write"
    secrets = ["vercel.token"]
  }

  param "id" {
    type        = "string"
    required    = true
    description = "Deployment ID to delete"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.vercel.com/v13/deployments/{{ args.id }}"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      teamId = "{{ args.teamId }}"
    }
  }

  result {
    decode = "json"
    output = "Deployment {{ args.id }} deleted."
  }
}

command "rollback_deployment" {
  title       = "Rollback deployment"
  summary     = "Rollback a project to a previous deployment"
  description = "Rollback a project's production deployment to a previously successful deployment."
  categories  = ["write", "deployments"]

  annotations {
    mode    = "write"
    secrets = ["vercel.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID"
  }

  param "deployment_id" {
    type        = "string"
    required    = true
    description = "Deployment ID to rollback to"
  }

  param "description" {
    type        = "string"
    required    = false
    description = "Reason for the rollback"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.vercel.com/v1/projects/{{ args.project_id }}/rollback/{{ args.deployment_id }}"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      teamId = "{{ args.teamId }}"
    }

    body {
      kind = "json"
      value = {
        description = "{{ args.description }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Rolled back project {{ args.project_id }} to deployment {{ args.deployment_id }}."
  }
}

command "list_env_vars" {
  title       = "List environment variables"
  summary     = "List environment variables for a project"
  description = "Retrieve all environment variables configured for a specific project."
  categories  = ["read", "projects"]

  annotations {
    mode    = "read"
    secrets = ["vercel.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID or name"
  }

  param "decrypt" {
    type        = "string"
    required    = false
    description = "Set to 'true' to decrypt secret values"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.vercel.com/v10/projects/{{ args.project_id }}/env"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      decrypt = "{{ args.decrypt }}"
      teamId  = "{{ args.teamId }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.envs | length }} environment variables for project {{ args.project_id }}."
  }
}

command "create_env_var" {
  title       = "Create environment variable"
  summary     = "Create an environment variable for a project"
  description = "Add a new environment variable to a Vercel project for the specified target environments."
  categories  = ["write", "projects"]

  annotations {
    mode    = "write"
    secrets = ["vercel.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID or name"
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

  param "type" {
    type        = "string"
    required    = true
    description = "Variable type: plain, secret, or system"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.vercel.com/v10/projects/{{ args.project_id }}/env"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      teamId = "{{ args.teamId }}"
    }

    body {
      kind = "json"
      value = {
        key   = "{{ args.key }}"
        value = "{{ args.value }}"
        type  = "{{ args.type }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Environment variable created: {{ result.created.key }} ({{ result.created.id }}), Type: {{ result.created.type }}"
  }
}

command "list_domains" {
  title       = "List domains"
  summary     = "List domains on the account"
  description = "Retrieve all domains registered with the Vercel account or team."
  categories  = ["read", "domains"]

  annotations {
    mode    = "read"
    secrets = ["vercel.token"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Maximum number of domains to return"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.vercel.com/v6/domains"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      limit  = "{{ args.limit }}"
      teamId = "{{ args.teamId }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.domains | length }} domains."
  }
}

command "get_domain" {
  title       = "Get domain"
  summary     = "Get details of a domain"
  description = "Retrieve detailed information about a specific domain including verification and nameserver status."
  categories  = ["read", "domains"]

  annotations {
    mode    = "read"
    secrets = ["vercel.token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Domain name"
  }

  param "teamId" {
    type        = "string"
    required    = false
    description = "Team identifier to scope the request"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.vercel.com/v6/domains/{{ args.domain }}"

    auth {
      kind   = "bearer"
      secret = "vercel.token"
    }

    query = {
      teamId = "{{ args.teamId }}"
    }
  }

  result {
    decode = "json"
    output = "Domain: {{ result.domain.name }}, Verified: {{ result.domain.verified }}, Created: {{ result.domain.createdAt }}"
  }
}
