version = 1
provider = "sentry"
categories = ["monitoring", "observability", "error-tracking"]

command "list_projects" {
  title       = "List projects"
  summary     = "List all projects in a Sentry organization"
  description = "Retrieve all projects belonging to the specified organization."
  categories  = ["projects"]

  annotations {
    mode    = "read"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://sentry.io/api/0/organizations/{{ args.organization }}/projects/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} projects."
  }
}

command "get_project" {
  title       = "Get project"
  summary     = "Get details of a specific project"
  description = "Retrieve detailed information about a Sentry project."
  categories  = ["projects"]

  annotations {
    mode    = "read"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  param "project" {
    type        = "string"
    required    = true
    description = "Project slug"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://sentry.io/api/0/projects/{{ args.organization }}/{{ args.project }}/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }
  }

  result {
    decode = "json"
    output = "{{ result.name }} ({{ result.slug }}) | Platform: {{ result.platform }} | Created: {{ result.dateCreated }}"
  }
}

command "create_project" {
  title       = "Create project"
  summary     = "Create a new project under a team"
  description = "Create a new Sentry project under the specified team in an organization."
  categories  = ["projects"]

  annotations {
    mode    = "write"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  param "team" {
    type        = "string"
    required    = true
    description = "Team slug to create the project under"
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Project name"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://sentry.io/api/0/teams/{{ args.organization }}/{{ args.team }}/projects/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }

    body {
      kind = "json"
      value = {
        name = "{{ args.name }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created project {{ result.name }} ({{ result.slug }}) with ID {{ result.id }}."
  }
}

command "delete_project" {
  title       = "Delete project"
  summary     = "Delete a project from an organization"
  description = "Permanently delete a Sentry project and all its data."
  categories  = ["projects"]

  annotations {
    mode    = "write"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  param "project" {
    type        = "string"
    required    = true
    description = "Project slug"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://sentry.io/api/0/projects/{{ args.organization }}/{{ args.project }}/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }
  }

  result {
    decode = "json"
    output = "Deleted project {{ args.project }} from organization {{ args.organization }}."
  }
}

command "list_issues" {
  title       = "List issues"
  summary     = "Search and list issues in an organization"
  description = "List issues across an organization with optional search query, sorting, and filtering."
  categories  = ["issues"]

  annotations {
    mode    = "read"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  param "query" {
    type        = "string"
    required    = false
    default     = "is:unresolved"
    description = "Search query (e.g. 'is:unresolved', 'assigned:me', 'level:error')"
  }

  param "sort" {
    type        = "string"
    required    = false
    default     = "date"
    description = "Sort by: date, freq, new, trends, or user"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 25
    description = "Maximum number of results (up to 100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://sentry.io/api/0/organizations/{{ args.organization }}/issues/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }

    query = {
      query = "{{ args.query }}"
      sort  = "{{ args.sort }}"
      limit = "{{ args.limit }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} issues."
  }
}

command "get_issue" {
  title       = "Get issue"
  summary     = "Get details of a specific issue"
  description = "Retrieve detailed information about a Sentry issue including event count, user count, and assignment."
  categories  = ["issues"]

  annotations {
    mode    = "read"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  param "issue_id" {
    type        = "string"
    required    = true
    description = "Numeric issue ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://sentry.io/api/0/organizations/{{ args.organization }}/issues/{{ args.issue_id }}/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }
  }

  result {
    decode = "json"
    output = "[{{ result.shortId }}] {{ result.title }} | Status: {{ result.status }} | Level: {{ result.level }} | Events: {{ result.count }}"
  }
}

command "update_issue" {
  title       = "Update issue"
  summary     = "Update the status of an issue"
  description = "Update a Sentry issue status to resolved, unresolved, ignored, or resolvedInNextRelease."
  categories  = ["issues"]

  annotations {
    mode    = "write"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  param "issue_id" {
    type        = "string"
    required    = true
    description = "Numeric issue ID"
  }

  param "status" {
    type        = "string"
    required    = true
    description = "New status: resolved, resolvedInNextRelease, unresolved, or ignored"
  }

  operation {
    protocol = "http"
    method   = "PUT"
    url      = "https://sentry.io/api/0/organizations/{{ args.organization }}/issues/{{ args.issue_id }}/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }

    body {
      kind = "json"
      value = {
        status = "{{ args.status }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Updated issue [{{ result.shortId }}] to status {{ result.status }}."
  }
}

command "list_issue_events" {
  title       = "List issue events"
  summary     = "List events for a specific issue"
  description = "Retrieve the list of events associated with a Sentry issue."
  categories  = ["issues"]

  annotations {
    mode    = "read"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  param "issue_id" {
    type        = "string"
    required    = true
    description = "Numeric issue ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://sentry.io/api/0/organizations/{{ args.organization }}/issues/{{ args.issue_id }}/events/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} events for issue {{ args.issue_id }}."
  }
}

command "get_event" {
  title       = "Get event"
  summary     = "Get full event details with stacktrace"
  description = "Retrieve the full details of an event including tags, user info, and exception stacktrace."
  categories  = ["events"]

  annotations {
    mode    = "read"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  param "project" {
    type        = "string"
    required    = true
    description = "Project slug"
  }

  param "event_id" {
    type        = "string"
    required    = true
    description = "Hexadecimal event ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://sentry.io/api/0/projects/{{ args.organization }}/{{ args.project }}/events/{{ args.event_id }}/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }
  }

  result {
    decode = "json"
    output = "Event {{ result.eventID }} | {{ result.title }} | Platform: {{ result.platform }} | Received: {{ result.dateReceived }}"
  }
}

command "list_releases" {
  title       = "List releases"
  summary     = "List releases for an organization"
  description = "Retrieve all releases for a Sentry organization, optionally filtered by version prefix."
  categories  = ["releases"]

  annotations {
    mode    = "read"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  param "query" {
    type        = "string"
    required    = false
    default     = ""
    description = "Version prefix to filter by"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://sentry.io/api/0/organizations/{{ args.organization }}/releases/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }

    query = {
      query = "{{ args.query }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} releases."
  }
}

command "create_release" {
  title       = "Create release"
  summary     = "Create a new release"
  description = "Create a new release for a Sentry organization with the specified version and project slugs."
  categories  = ["releases"]

  annotations {
    mode    = "write"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  param "version" {
    type        = "string"
    required    = true
    description = "Release version identifier (semver, SHA, etc.)"
  }

  param "projects" {
    type        = "array"
    required    = true
    description = "List of project slugs for this release"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://sentry.io/api/0/organizations/{{ args.organization }}/releases/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }

    body {
      kind = "json"
      value = {
        version  = "{{ args.version }}"
        projects = "{{ args.projects }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created release {{ result.version }}."
  }
}

command "create_deploy" {
  title       = "Create deploy"
  summary     = "Record a deployment for a release"
  description = "Record a deployment of a release to an environment in Sentry."
  categories  = ["releases"]

  annotations {
    mode    = "write"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  param "version" {
    type        = "string"
    required    = true
    description = "Release version to deploy"
  }

  param "environment" {
    type        = "string"
    required    = true
    description = "Target environment (e.g. production, staging)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://sentry.io/api/0/organizations/{{ args.organization }}/releases/{{ args.version }}/deploys/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }

    body {
      kind = "json"
      value = {
        environment = "{{ args.environment }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Deployed release {{ args.version }} to {{ result.environment }}."
  }
}

command "list_teams" {
  title       = "List teams"
  summary     = "List teams in an organization"
  description = "Retrieve all teams belonging to a Sentry organization."
  categories  = ["teams"]

  annotations {
    mode    = "read"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://sentry.io/api/0/organizations/{{ args.organization }}/teams/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} teams."
  }
}

command "resolve_short_id" {
  title       = "Resolve short ID"
  summary     = "Resolve a short issue ID to its full details"
  description = "Resolve a Sentry short ID (e.g. PROJECT-123) to the full issue ID and details."
  categories  = ["issues"]

  annotations {
    mode    = "read"
    secrets = ["sentry.auth_token"]
  }

  param "organization" {
    type        = "string"
    required    = true
    description = "Organization slug"
  }

  param "short_id" {
    type        = "string"
    required    = true
    description = "Short issue ID (e.g. PROJECT-123)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://sentry.io/api/0/organizations/{{ args.organization }}/shortids/{{ args.short_id }}/"

    auth {
      kind   = "bearer"
      secret = "sentry.auth_token"
    }
  }

  result {
    decode = "json"
    output = "{{ result.shortId }} -> Issue {{ result.groupId }} in {{ result.projectSlug }}: {{ result.group.title }} ({{ result.group.status }})"
  }
}
