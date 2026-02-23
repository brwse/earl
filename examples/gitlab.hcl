version = 1
provider = "gitlab"
categories = ["devops", "version-control", "ci-cd"]

command "get_current_user" {
  title       = "Get current user"
  summary     = "Get the currently authenticated GitLab user"
  description = "Retrieve profile information for the user associated with the provided token."
  categories  = ["users"]

  annotations {
    mode    = "read"
    secrets = ["gitlab.token"]
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://gitlab.com/api/v4/user"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }
  }

  result {
    decode = "json"
    output = "{{ result.username }} ({{ result.name }}) — ID: {{ result.id }}, email: {{ result.email }}, state: {{ result.state }}"
  }
}

command "list_projects" {
  title       = "List projects"
  summary     = "List GitLab projects visible to the authenticated user"
  description = "Search and list projects. Filter by ownership, visibility, or search term."
  categories  = ["projects"]

  annotations {
    mode    = "read"
    secrets = ["gitlab.token"]
  }

  param "search" {
    type        = "string"
    required    = false
    description = "Search by name, path, or description"
  }

  param "owned" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Only return projects owned by the current user"
  }

  param "visibility" {
    type        = "string"
    required    = false
    description = "Filter by visibility: public, internal, or private"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of results per page (max 100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://gitlab.com/api/v4/projects"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }

    query = {
      search     = "{{ args.search }}"
      owned      = "{{ args.owned }}"
      visibility = "{{ args.visibility }}"
      per_page   = "{{ args.per_page }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} projects."
  }
}

command "get_project" {
  title       = "Get project"
  summary     = "Get details of a specific GitLab project"
  description = "Retrieve detailed information about a project by its numeric ID or URL-encoded path (e.g. my-group%2Fmy-project)."
  categories  = ["projects"]

  annotations {
    mode    = "read"
    secrets = ["gitlab.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID (integer) or URL-encoded namespace/path"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://gitlab.com/api/v4/projects/{{ args.project_id }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }
  }

  result {
    decode = "json"
    output = "{{ result.path_with_namespace }} (ID: {{ result.id }}) [{{ result.visibility }}] — default branch: {{ result.default_branch }}, stars: {{ result.star_count }}, forks: {{ result.forks_count }} — {{ result.web_url }}"
  }
}

command "create_project" {
  title       = "Create project"
  summary     = "Create a new GitLab project"
  description = "Create a new project. Optionally specify visibility, namespace, and whether to initialize with a README."
  categories  = ["projects"]

  annotations {
    mode    = "write"
    secrets = ["gitlab.token"]
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Project name"
  }

  param "description" {
    type        = "string"
    required    = false
    description = "Project description"
  }

  param "visibility" {
    type        = "string"
    required    = false
    default     = "private"
    description = "Visibility level: private, internal, or public"
  }

  param "namespace_id" {
    type        = "integer"
    required    = false
    description = "Group or namespace ID to create the project in"
  }

  param "initialize_with_readme" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Initialize the repository with a README"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://gitlab.com/api/v4/projects"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }

    body {
      kind = "json"
      value = {
        name                     = "{{ args.name }}"
        description              = "{{ args.description }}"
        visibility               = "{{ args.visibility }}"
        namespace_id             = "{{ args.namespace_id }}"
        initialize_with_readme   = "{{ args.initialize_with_readme }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created project {{ result.path_with_namespace }} (ID: {{ result.id }}) [{{ result.visibility }}] — {{ result.web_url }}"
  }
}

command "list_issues" {
  title       = "List issues"
  summary     = "List issues for a GitLab project"
  description = "List issues in a project with optional filters for state, labels, assignee, and milestone."
  categories  = ["issues"]

  annotations {
    mode    = "read"
    secrets = ["gitlab.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID or URL-encoded path"
  }

  param "state" {
    type        = "string"
    required    = false
    default     = "opened"
    description = "Filter by state: opened, closed, or all"
  }

  param "labels" {
    type        = "string"
    required    = false
    description = "Comma-separated label names to filter by"
  }

  param "search" {
    type        = "string"
    required    = false
    description = "Search in title and description"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of results per page (max 100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://gitlab.com/api/v4/projects/{{ args.project_id }}/issues"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }

    query = {
      state    = "{{ args.state }}"
      labels   = "{{ args.labels }}"
      search   = "{{ args.search }}"
      per_page = "{{ args.per_page }}"
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
  description = "Retrieve detailed information about a single issue by its project-scoped IID."
  categories  = ["issues"]

  annotations {
    mode    = "read"
    secrets = ["gitlab.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID or URL-encoded path"
  }

  param "issue_iid" {
    type        = "integer"
    required    = true
    description = "Project-scoped issue IID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://gitlab.com/api/v4/projects/{{ args.project_id }}/issues/{{ args.issue_iid }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }
  }

  result {
    decode = "json"
    output = "Issue #{{ result.iid }}: {{ result.title }} [{{ result.state }}] — author: {{ result.author.username }}, labels: {{ result.labels | join(\", \") }}, {{ result.web_url }}"
  }
}

command "create_issue" {
  title       = "Create issue"
  summary     = "Create a new issue in a GitLab project"
  description = "Create a new issue with a title and optional description, labels, and assignees."
  categories  = ["issues"]

  annotations {
    mode    = "write"
    secrets = ["gitlab.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID or URL-encoded path"
  }

  param "title" {
    type        = "string"
    required    = true
    description = "Issue title"
  }

  param "description" {
    type        = "string"
    required    = false
    description = "Issue description (supports Markdown)"
  }

  param "labels" {
    type        = "string"
    required    = false
    description = "Comma-separated label names"
  }

  param "assignee_ids" {
    type        = "string"
    required    = false
    description = "Comma-separated user IDs to assign"
  }

  param "confidential" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Mark as confidential"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://gitlab.com/api/v4/projects/{{ args.project_id }}/issues"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }

    body {
      kind = "json"
      value = {
        title         = "{{ args.title }}"
        description   = "{{ args.description }}"
        labels        = "{{ args.labels }}"
        assignee_ids  = "{{ args.assignee_ids }}"
        confidential  = "{{ args.confidential }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created issue #{{ result.iid }}: {{ result.title }} — {{ result.web_url }}"
  }
}

command "update_issue" {
  title       = "Update issue"
  summary     = "Update an existing issue"
  description = "Update an issue's title, description, state, labels, or assignees."
  categories  = ["issues"]

  annotations {
    mode    = "write"
    secrets = ["gitlab.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID or URL-encoded path"
  }

  param "issue_iid" {
    type        = "integer"
    required    = true
    description = "Project-scoped issue IID"
  }

  param "title" {
    type        = "string"
    required    = false
    description = "New issue title"
  }

  param "description" {
    type        = "string"
    required    = false
    description = "New issue description"
  }

  param "state_event" {
    type        = "string"
    required    = false
    description = "Change state: close or reopen"
  }

  param "labels" {
    type        = "string"
    required    = false
    description = "Replace all labels (comma-separated names)"
  }

  param "add_labels" {
    type        = "string"
    required    = false
    description = "Labels to add (comma-separated)"
  }

  operation {
    protocol = "http"
    method   = "PUT"
    url      = "https://gitlab.com/api/v4/projects/{{ args.project_id }}/issues/{{ args.issue_iid }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }

    body {
      kind = "json"
      value = {
        title       = "{{ args.title }}"
        description = "{{ args.description }}"
        state_event = "{{ args.state_event }}"
        labels      = "{{ args.labels }}"
        add_labels  = "{{ args.add_labels }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Updated issue #{{ result.iid }}: {{ result.title }} [{{ result.state }}] — {{ result.web_url }}"
  }
}

command "list_merge_requests" {
  title       = "List merge requests"
  summary     = "List merge requests for a GitLab project"
  description = "List merge requests with optional filters for state, scope, labels, and search."
  categories  = ["merge-requests"]

  annotations {
    mode    = "read"
    secrets = ["gitlab.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID or URL-encoded path"
  }

  param "state" {
    type        = "string"
    required    = false
    default     = "opened"
    description = "Filter by state: opened, closed, merged, locked, or all"
  }

  param "scope" {
    type        = "string"
    required    = false
    description = "Filter by scope: created_by_me, assigned_to_me, or all"
  }

  param "labels" {
    type        = "string"
    required    = false
    description = "Comma-separated label names to filter by"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of results per page (max 100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://gitlab.com/api/v4/projects/{{ args.project_id }}/merge_requests"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }

    query = {
      state    = "{{ args.state }}"
      scope    = "{{ args.scope }}"
      labels   = "{{ args.labels }}"
      per_page = "{{ args.per_page }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} merge requests."
  }
}

command "get_merge_request" {
  title       = "Get merge request"
  summary     = "Get details of a specific merge request"
  description = "Retrieve detailed information about a merge request including its status, branches, and pipeline state."
  categories  = ["merge-requests"]

  annotations {
    mode    = "read"
    secrets = ["gitlab.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID or URL-encoded path"
  }

  param "merge_request_iid" {
    type        = "integer"
    required    = true
    description = "Project-scoped merge request IID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://gitlab.com/api/v4/projects/{{ args.project_id }}/merge_requests/{{ args.merge_request_iid }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }
  }

  result {
    decode = "json"
    output = "MR !{{ result.iid }}: {{ result.title }} [{{ result.state }}] — {{ result.source_branch }} -> {{ result.target_branch }}, author: {{ result.author.username }}, conflicts: {{ result.has_conflicts }}, {{ result.web_url }}"
  }
}

command "create_merge_request" {
  title       = "Create merge request"
  summary     = "Create a new merge request"
  description = "Create a merge request from a source branch to a target branch with optional reviewers and labels."
  categories  = ["merge-requests"]

  annotations {
    mode    = "write"
    secrets = ["gitlab.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID or URL-encoded path"
  }

  param "source_branch" {
    type        = "string"
    required    = true
    description = "Source branch name"
  }

  param "target_branch" {
    type        = "string"
    required    = true
    description = "Target branch name"
  }

  param "title" {
    type        = "string"
    required    = true
    description = "Merge request title"
  }

  param "description" {
    type        = "string"
    required    = false
    description = "Merge request description (supports Markdown)"
  }

  param "draft" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Mark as draft"
  }

  param "squash" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Squash commits when merging"
  }

  param "remove_source_branch" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Delete source branch after merge"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://gitlab.com/api/v4/projects/{{ args.project_id }}/merge_requests"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }

    body {
      kind = "json"
      value = {
        source_branch        = "{{ args.source_branch }}"
        target_branch        = "{{ args.target_branch }}"
        title                = "{{ args.title }}"
        description          = "{{ args.description }}"
        draft                = "{{ args.draft }}"
        squash               = "{{ args.squash }}"
        remove_source_branch = "{{ args.remove_source_branch }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created MR !{{ result.iid }}: {{ result.title }} ({{ result.source_branch }} -> {{ result.target_branch }}) — {{ result.web_url }}"
  }
}

command "merge_merge_request" {
  title       = "Merge a merge request"
  summary     = "Merge an open merge request"
  description = "Accept and merge a merge request. Optionally squash commits, delete the source branch, or set auto-merge when the pipeline succeeds."
  categories  = ["merge-requests"]

  annotations {
    mode    = "write"
    secrets = ["gitlab.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID or URL-encoded path"
  }

  param "merge_request_iid" {
    type        = "integer"
    required    = true
    description = "Project-scoped merge request IID"
  }

  param "squash" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Squash commits on merge"
  }

  param "should_remove_source_branch" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Delete source branch after merge"
  }

  param "merge_when_pipeline_succeeds" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Auto-merge when the pipeline passes"
  }

  operation {
    protocol = "http"
    method   = "PUT"
    url      = "https://gitlab.com/api/v4/projects/{{ args.project_id }}/merge_requests/{{ args.merge_request_iid }}/merge"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }

    body {
      kind = "json"
      value = {
        squash                       = "{{ args.squash }}"
        should_remove_source_branch  = "{{ args.should_remove_source_branch }}"
        merge_when_pipeline_succeeds = "{{ args.merge_when_pipeline_succeeds }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Merged !{{ result.iid }}: {{ result.title }} ({{ result.source_branch }} -> {{ result.target_branch }}) — merge commit: {{ result.merge_commit_sha }}"
  }
}

command "list_pipelines" {
  title       = "List pipelines"
  summary     = "List CI/CD pipelines for a project"
  description = "List pipelines with optional filters for status, ref, and scope."
  categories  = ["ci-cd"]

  annotations {
    mode    = "read"
    secrets = ["gitlab.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID or URL-encoded path"
  }

  param "status" {
    type        = "string"
    required    = false
    description = "Filter by status: running, pending, success, failed, canceled, skipped, or manual"
  }

  param "ref" {
    type        = "string"
    required    = false
    description = "Filter by branch or tag name"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of results per page (max 100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://gitlab.com/api/v4/projects/{{ args.project_id }}/pipelines"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }

    query = {
      status   = "{{ args.status }}"
      ref      = "{{ args.ref }}"
      per_page = "{{ args.per_page }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} pipelines."
  }
}

command "create_pipeline" {
  title       = "Create pipeline"
  summary     = "Trigger a new CI/CD pipeline"
  description = "Create and trigger a new pipeline on a given branch or tag."
  categories  = ["ci-cd"]

  annotations {
    mode    = "write"
    secrets = ["gitlab.token"]
  }

  param "project_id" {
    type        = "string"
    required    = true
    description = "Project ID or URL-encoded path"
  }

  param "ref" {
    type        = "string"
    required    = true
    description = "Branch or tag to run the pipeline on"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://gitlab.com/api/v4/projects/{{ args.project_id }}/pipeline"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }

    body {
      kind = "json"
      value = {
        ref = "{{ args.ref }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created pipeline #{{ result.id }} [{{ result.status }}] on ref {{ result.ref }} — {{ result.web_url }}"
  }
}

command "list_groups" {
  title       = "List groups"
  summary     = "List GitLab groups visible to the authenticated user"
  description = "List groups with optional filters for search, ownership, and visibility."
  categories  = ["groups"]

  annotations {
    mode    = "read"
    secrets = ["gitlab.token"]
  }

  param "search" {
    type        = "string"
    required    = false
    description = "Filter by group name or path"
  }

  param "owned" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Only return groups owned by the current user"
  }

  param "visibility" {
    type        = "string"
    required    = false
    description = "Filter by visibility: public, internal, or private"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of results per page (max 100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://gitlab.com/api/v4/groups"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "PRIVATE-TOKEN"
      secret   = "gitlab.token"
    }

    query = {
      search     = "{{ args.search }}"
      owned      = "{{ args.owned }}"
      visibility = "{{ args.visibility }}"
      per_page   = "{{ args.per_page }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} groups."
  }
}
