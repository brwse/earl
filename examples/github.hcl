version = 1
provider = "github"
categories = ["scm", "issues", "ci-cd"]

command "list_repos" {
  title       = "List repositories"
  summary     = "List repositories for the authenticated user"
  description = "Fetch repositories for the authenticated user, with optional filtering by type and sorting."
  categories  = ["repos"]

  annotations {
    mode    = "read"
    secrets = ["github.token"]
  }

  param "type" {
    type        = "string"
    required    = false
    default     = "all"
    description = "Filter by type: all, public, private, forks, sources, member"
  }

  param "sort" {
    type        = "string"
    required    = false
    default     = "updated"
    description = "Sort by: created, updated, pushed, full_name"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 30
    description = "Number of results per page (max 100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.github.com/user/repos"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    query = {
      type     = "{{ args.type }}"
      sort     = "{{ args.sort }}"
      per_page = "{{ args.per_page }}"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} repositories:\n{% for repo in result %}  - {{ repo.full_name }} ({{ repo.visibility }}, stars: {{ repo.stargazers_count }}){% if repo.description %} — {{ repo.description }}{% endif %}\n{% endfor %}"
  }
}

command "get_repo" {
  title       = "Get repository"
  summary     = "Get details about a specific repository"
  description = "Fetch detailed information about a GitHub repository including stars, forks, language, and description."
  categories  = ["repos"]

  annotations {
    mode    = "read"
    secrets = ["github.token"]
  }

  param "owner" {
    type        = "string"
    required    = true
    description = "Repository owner (user or organization)"
  }

  param "repo" {
    type        = "string"
    required    = true
    description = "Repository name"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.github.com/repos/{{ args.owner }}/{{ args.repo }}"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }
  }

  result {
    decode = "json"
    output = "{{ result.full_name }} ({{ result.visibility }})\nStars: {{ result.stargazers_count }}  Forks: {{ result.forks_count }}  Watchers: {{ result.watchers_count }}\nLanguage: {{ result.language | default('N/A') }}\nDefault branch: {{ result.default_branch }}\nDescription: {{ result.description | default('None') }}\nURL: {{ result.html_url }}"
  }
}

command "create_repo" {
  title       = "Create repository"
  summary     = "Create a new repository for the authenticated user"
  description = "Create a new GitHub repository with optional description, visibility, and initialization settings."
  categories  = ["repos"]

  annotations {
    mode    = "write"
    secrets = ["github.token"]
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Repository name"
  }

  param "description" {
    type        = "string"
    required    = false
    default     = ""
    description = "Repository description"
  }

  param "private" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Whether the repository should be private"
  }

  param "auto_init" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Initialize with a README"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.github.com/user/repos"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }

    body {
      kind = "json"
      value = {
        name        = "{{ args.name }}"
        description = "{{ args.description }}"
        private     = "{{ args.private }}"
        auto_init   = "{{ args.auto_init }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created repository: {{ result.full_name }}\nURL: {{ result.html_url }}\nVisibility: {{ result.visibility }}\nClone: {{ result.clone_url }}"
  }
}

command "search_repos" {
  title       = "Search repositories"
  summary     = "Search GitHub repositories by query"
  description = "Search repositories using GitHub's search API. Supports qualifiers like language:rust, stars:>100, topic:cli."
  categories  = ["search", "repos"]

  annotations {
    mode    = "read"
    secrets = ["github.token"]
  }

  param "query" {
    type        = "string"
    required    = true
    description = "Search query (e.g. 'language:rust stars:>100 topic:cli')"
  }

  param "sort" {
    type        = "string"
    required    = false
    default     = "stars"
    description = "Sort by: stars, forks, help-wanted-issues, updated"
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
    url      = "https://api.github.com/search/repositories"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    query = {
      q        = "{{ args.query }}"
      sort     = "{{ args.sort }}"
      order    = "desc"
      per_page = "{{ args.per_page }}"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.total_count }} repositories:\n{% for repo in result.items[:10] %}  - {{ repo.full_name }} (stars: {{ repo.stargazers_count }}) — {{ repo.description | default('No description') }}\n{% endfor %}"
  }
}

command "list_issues" {
  title       = "List issues"
  summary     = "List issues in a repository"
  description = "Fetch issues for a repository with optional filtering by state, labels, and assignee."
  categories  = ["issues"]

  annotations {
    mode    = "read"
    secrets = ["github.token"]
  }

  param "owner" {
    type        = "string"
    required    = true
    description = "Repository owner"
  }

  param "repo" {
    type        = "string"
    required    = true
    description = "Repository name"
  }

  param "state" {
    type        = "string"
    required    = false
    default     = "open"
    description = "Filter by state: open, closed, all"
  }

  param "labels" {
    type        = "string"
    required    = false
    description = "Comma-separated list of label names"
  }

  param "sort" {
    type        = "string"
    required    = false
    default     = "created"
    description = "Sort by: created, updated, comments"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 30
    description = "Number of results per page (max 100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.github.com/repos/{{ args.owner }}/{{ args.repo }}/issues"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    query = {
      state    = "{{ args.state }}"
      labels   = "{{ args.labels }}"
      sort     = "{{ args.sort }}"
      per_page = "{{ args.per_page }}"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} issues in {{ args.owner }}/{{ args.repo }}:\n{% for issue in result %}{% if not issue.pull_request %}  #{{ issue.number }} [{{ issue.state }}] {{ issue.title }}{% if issue.assignee %} (assigned: {{ issue.assignee.login }}){% endif %}\n{% endif %}{% endfor %}"
  }
}

command "get_issue" {
  title       = "Get issue"
  summary     = "Get details about a specific issue"
  description = "Fetch detailed information about a single issue including labels, assignees, and body."
  categories  = ["issues"]

  annotations {
    mode    = "read"
    secrets = ["github.token"]
  }

  param "owner" {
    type        = "string"
    required    = true
    description = "Repository owner"
  }

  param "repo" {
    type        = "string"
    required    = true
    description = "Repository name"
  }

  param "issue_number" {
    type        = "integer"
    required    = true
    description = "Issue number"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.github.com/repos/{{ args.owner }}/{{ args.repo }}/issues/{{ args.issue_number }}"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }
  }

  result {
    decode = "json"
    output = "#{{ result.number }} [{{ result.state }}] {{ result.title }}\nAuthor: {{ result.user.login }}  Created: {{ result.created_at }}\nLabels: {{ result.labels | map(attribute='name') | join(', ') | default('none') }}\nAssignees: {{ result.assignees | map(attribute='login') | join(', ') | default('unassigned') }}\n\n{{ result.body | default('No description.') }}"
  }
}

command "create_issue" {
  title       = "Create issue"
  summary     = "Create a new issue in a repository"
  description = "Create a GitHub issue with a title and optional body, labels, and assignees."
  categories  = ["issues"]

  annotations {
    mode    = "write"
    secrets = ["github.token"]
  }

  param "owner" {
    type        = "string"
    required    = true
    description = "Repository owner"
  }

  param "repo" {
    type        = "string"
    required    = true
    description = "Repository name"
  }

  param "title" {
    type        = "string"
    required    = true
    description = "Issue title"
  }

  param "body" {
    type        = "string"
    required    = false
    default     = ""
    description = "Issue body (Markdown)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.github.com/repos/{{ args.owner }}/{{ args.repo }}/issues"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }

    body {
      kind = "json"
      value = {
        title = "{{ args.title }}"
        body  = "{{ args.body }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created issue #{{ result.number }}: {{ result.title }}\nURL: {{ result.html_url }}"
  }
}

command "update_issue" {
  title       = "Update issue"
  summary     = "Update an existing issue"
  description = "Update the title, body, or state of an existing issue."
  categories  = ["issues"]

  annotations {
    mode    = "write"
    secrets = ["github.token"]
  }

  param "owner" {
    type        = "string"
    required    = true
    description = "Repository owner"
  }

  param "repo" {
    type        = "string"
    required    = true
    description = "Repository name"
  }

  param "issue_number" {
    type        = "integer"
    required    = true
    description = "Issue number"
  }

  param "state" {
    type        = "string"
    required    = false
    description = "New state: open or closed (omit to keep current)"
  }

  param "title" {
    type        = "string"
    required    = false
    description = "New title (omit to keep current)"
  }

  param "body" {
    type        = "string"
    required    = false
    description = "New body in Markdown (omit to keep current)"
  }

  operation {
    protocol = "http"
    method   = "PATCH"
    url      = "https://api.github.com/repos/{{ args.owner }}/{{ args.repo }}/issues/{{ args.issue_number }}"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }

    body {
      kind = "json"
      value = {
        state = "{{ args.state }}"
        title = "{{ args.title }}"
        body  = "{{ args.body }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Updated issue #{{ result.number }}: {{ result.title }} [{{ result.state }}]\nURL: {{ result.html_url }}"
  }
}

command "create_comment" {
  title       = "Create comment"
  summary     = "Add a comment to an issue or pull request"
  description = "Post a new comment on an issue or pull request. Pull requests are issues in GitHub's data model, so this works for both."
  categories  = ["issues"]

  annotations {
    mode    = "write"
    secrets = ["github.token"]
  }

  param "owner" {
    type        = "string"
    required    = true
    description = "Repository owner"
  }

  param "repo" {
    type        = "string"
    required    = true
    description = "Repository name"
  }

  param "issue_number" {
    type        = "integer"
    required    = true
    description = "Issue or pull request number"
  }

  param "body" {
    type        = "string"
    required    = true
    description = "Comment body (Markdown)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.github.com/repos/{{ args.owner }}/{{ args.repo }}/issues/{{ args.issue_number }}/comments"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }

    body {
      kind = "json"
      value = {
        body = "{{ args.body }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Comment added to #{{ args.issue_number }} by {{ result.user.login }}\nURL: {{ result.html_url }}"
  }
}

command "search_issues" {
  title       = "Search issues"
  summary     = "Search GitHub issues and pull requests by query"
  description = "Search issues and pull requests across repositories using GitHub's search API. Supports qualifiers like repo:owner/name, is:issue, state:open, label:bug."
  categories  = ["search", "issues"]

  annotations {
    mode    = "read"
    secrets = ["github.token"]
  }

  param "query" {
    type        = "string"
    required    = true
    description = "Search query (e.g. 'repo:owner/repo is:open label:bug')"
  }

  param "sort" {
    type        = "string"
    required    = false
    default     = "created"
    description = "Sort by: comments, reactions, interactions, created, updated"
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
    url      = "https://api.github.com/search/issues"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    query = {
      q        = "{{ args.query }}"
      sort     = "{{ args.sort }}"
      order    = "desc"
      per_page = "{{ args.per_page }}"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.total_count }} results:\n{% for item in result.items[:10] %}  #{{ item.number }} [{{ item.state }}] {{ item.title }}\n{% endfor %}"
  }
}

command "list_pulls" {
  title       = "List pull requests"
  summary     = "List pull requests in a repository"
  description = "Fetch pull requests for a repository with optional filtering by state, head branch, and base branch."
  categories  = ["pull-requests"]

  annotations {
    mode    = "read"
    secrets = ["github.token"]
  }

  param "owner" {
    type        = "string"
    required    = true
    description = "Repository owner"
  }

  param "repo" {
    type        = "string"
    required    = true
    description = "Repository name"
  }

  param "state" {
    type        = "string"
    required    = false
    default     = "open"
    description = "Filter by state: open, closed, all"
  }

  param "sort" {
    type        = "string"
    required    = false
    default     = "created"
    description = "Sort by: created, updated, popularity, long-running"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 30
    description = "Number of results per page (max 100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.github.com/repos/{{ args.owner }}/{{ args.repo }}/pulls"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    query = {
      state    = "{{ args.state }}"
      sort     = "{{ args.sort }}"
      per_page = "{{ args.per_page }}"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} pull requests:\n{% for pr in result %}  #{{ pr.number }} [{{ pr.state }}{% if pr.draft %}/draft{% endif %}] {{ pr.title }} ({{ pr.head.label }} -> {{ pr.base.label }})\n{% endfor %}"
  }
}

command "create_pull" {
  title       = "Create pull request"
  summary     = "Create a new pull request"
  description = "Create a pull request to merge changes from a head branch into a base branch."
  categories  = ["pull-requests"]

  annotations {
    mode    = "write"
    secrets = ["github.token"]
  }

  param "owner" {
    type        = "string"
    required    = true
    description = "Repository owner"
  }

  param "repo" {
    type        = "string"
    required    = true
    description = "Repository name"
  }

  param "title" {
    type        = "string"
    required    = true
    description = "Pull request title"
  }

  param "head" {
    type        = "string"
    required    = true
    description = "Source branch name"
  }

  param "base" {
    type        = "string"
    required    = true
    description = "Target branch name"
  }

  param "body" {
    type        = "string"
    required    = false
    default     = ""
    description = "Pull request body (Markdown)"
  }

  param "draft" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Create as a draft pull request"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.github.com/repos/{{ args.owner }}/{{ args.repo }}/pulls"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }

    body {
      kind = "json"
      value = {
        title = "{{ args.title }}"
        head  = "{{ args.head }}"
        base  = "{{ args.base }}"
        body  = "{{ args.body }}"
        draft = "{{ args.draft }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created PR #{{ result.number }}: {{ result.title }}\n{{ result.head.label }} -> {{ result.base.label }}\nURL: {{ result.html_url }}"
  }
}

command "merge_pull" {
  title       = "Merge pull request"
  summary     = "Merge a pull request"
  description = "Merge a pull request using the specified merge method (merge, squash, or rebase)."
  categories  = ["pull-requests"]

  annotations {
    mode    = "write"
    secrets = ["github.token"]
  }

  param "owner" {
    type        = "string"
    required    = true
    description = "Repository owner"
  }

  param "repo" {
    type        = "string"
    required    = true
    description = "Repository name"
  }

  param "pull_number" {
    type        = "integer"
    required    = true
    description = "Pull request number"
  }

  param "merge_method" {
    type        = "string"
    required    = false
    default     = "merge"
    description = "Merge method: merge, squash, rebase"
  }

  param "commit_title" {
    type        = "string"
    required    = false
    description = "Custom commit title for squash or merge commits (omit to use PR title)"
  }

  operation {
    protocol = "http"
    method   = "PUT"
    url      = "https://api.github.com/repos/{{ args.owner }}/{{ args.repo }}/pulls/{{ args.pull_number }}/merge"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }

    body {
      kind = "json"
      value = {
        merge_method = "{{ args.merge_method }}"
        commit_title = "{{ args.commit_title }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Merged PR #{{ args.pull_number }}: {{ result.message }}\nSHA: {{ result.sha }}"
  }
}

command "list_workflow_runs" {
  title       = "List workflow runs"
  summary     = "List GitHub Actions workflow runs for a repository"
  description = "Fetch recent workflow runs with optional filtering by branch, event type, and status."
  categories  = ["ci-cd"]

  annotations {
    mode    = "read"
    secrets = ["github.token"]
  }

  param "owner" {
    type        = "string"
    required    = true
    description = "Repository owner"
  }

  param "repo" {
    type        = "string"
    required    = true
    description = "Repository name"
  }

  param "branch" {
    type        = "string"
    required    = false
    description = "Filter by branch name"
  }

  param "status" {
    type        = "string"
    required    = false
    description = "Filter by status: completed, success, failure, cancelled, in_progress, queued"
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
    url      = "https://api.github.com/repos/{{ args.owner }}/{{ args.repo }}/actions/runs"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    query = {
      branch   = "{{ args.branch }}"
      status   = "{{ args.status }}"
      per_page = "{{ args.per_page }}"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
    }
  }

  result {
    decode = "json"
    output = "Workflow runs for {{ args.owner }}/{{ args.repo }}:\n{% for run in result.workflow_runs[:10] %}  #{{ run.id }} {{ run.name }} [{{ run.conclusion | default(run.status) }}] on {{ run.head_branch }} ({{ run.created_at }})\n{% endfor %}"
  }
}

command "trigger_workflow" {
  title       = "Trigger workflow"
  summary     = "Manually trigger a GitHub Actions workflow"
  description = "Dispatch a workflow_dispatch event to trigger a GitHub Actions workflow on the specified branch or tag."
  categories  = ["ci-cd"]

  annotations {
    mode    = "write"
    secrets = ["github.token"]
  }

  param "owner" {
    type        = "string"
    required    = true
    description = "Repository owner"
  }

  param "repo" {
    type        = "string"
    required    = true
    description = "Repository name"
  }

  param "workflow_id" {
    type        = "string"
    required    = true
    description = "Workflow filename (e.g. 'ci.yml') or numeric ID"
  }

  param "ref" {
    type        = "string"
    required    = true
    description = "Branch or tag to run the workflow on"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.github.com/repos/{{ args.owner }}/{{ args.repo }}/actions/workflows/{{ args.workflow_id }}/dispatches"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    headers = {
      Accept               = "application/vnd.github+json"
      User-Agent           = "earl"
      X-GitHub-Api-Version = "2022-11-28"
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
    output = "Triggered workflow {{ args.workflow_id }} on {{ args.ref }} for {{ args.owner }}/{{ args.repo }}.\nNote: GitHub returns 204 No Content on success. Check workflow runs to monitor status."
  }
}
