version = 1
provider = "jira"
categories = ["project-management", "issue-tracking", "agile"]

command "search_issues" {
  title       = "Search issues"
  summary     = "Search Jira issues using JQL"
  description = "Search for issues across projects using Jira Query Language (JQL)."
  categories  = ["search", "issues"]

  annotations {
    mode    = "read"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain (e.g. 'mycompany' for mycompany.atlassian.net)"
  }

  param "jql" {
    type        = "string"
    required    = true
    description = "JQL query (e.g. 'project = PROJ AND status = \"In Progress\"')"
  }

  param "max_results" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Maximum number of results to return"
  }

  param "fields" {
    type        = "string"
    required    = false
    default     = "key,summary,status,assignee,priority,issuetype"
    description = "Comma-separated list of fields to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.domain }}.atlassian.net/rest/api/2/search"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    query = {
      jql        = "{{ args.jql }}"
      maxResults = "{{ args.max_results }}"
      fields     = "{{ args.fields }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.total }} issues matching the query."
  }
}

command "get_issue" {
  title       = "Get issue"
  summary     = "Get details of a Jira issue"
  description = "Retrieve full details of a Jira issue by its key."
  categories  = ["issues"]

  annotations {
    mode    = "read"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  param "issue_key" {
    type        = "string"
    required    = true
    description = "Issue key (e.g. 'PROJ-123')"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.domain }}.atlassian.net/rest/api/2/issue/{{ args.issue_key }}"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "{{ result.key }}: {{ result.fields.summary }}\nType: {{ result.fields.issuetype.name }}  Status: {{ result.fields.status.name }}  Priority: {{ result.fields.priority.name }}\nAssignee: {{ result.fields.assignee.displayName | default('Unassigned') }}\nReporter: {{ result.fields.reporter.displayName }}"
  }
}

command "create_issue" {
  title       = "Create issue"
  summary     = "Create a new Jira issue"
  description = "Create a new issue in a Jira project."
  categories  = ["write", "issues"]

  annotations {
    mode    = "write"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  param "project_key" {
    type        = "string"
    required    = true
    description = "Project key (e.g. 'PROJ')"
  }

  param "summary" {
    type        = "string"
    required    = true
    description = "Issue title/summary"
  }

  param "issue_type" {
    type        = "string"
    required    = true
    description = "Issue type (e.g. 'Bug', 'Story', 'Task')"
  }

  param "description" {
    type        = "string"
    required    = false
    default     = ""
    description = "Issue description (plain text)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ args.domain }}.atlassian.net/rest/api/2/issue"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        fields = {
          project = {
            key = "{{ args.project_key }}"
          }
          summary = "{{ args.summary }}"
          issuetype = {
            name = "{{ args.issue_type }}"
          }
          description = "{{ args.description }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Created issue {{ result.key }}\nURL: https://{{ args.domain }}.atlassian.net/browse/{{ result.key }}"
  }
}

command "update_issue" {
  title       = "Update issue"
  summary     = "Update the summary of a Jira issue"
  description = "Update the summary (title) of an existing Jira issue."
  categories  = ["write", "issues"]

  annotations {
    mode    = "write"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  param "issue_key" {
    type        = "string"
    required    = true
    description = "Issue key to update"
  }

  param "summary" {
    type        = "string"
    required    = true
    description = "New issue summary"
  }

  operation {
    protocol = "http"
    method   = "PUT"
    url      = "https://{{ args.domain }}.atlassian.net/rest/api/2/issue/{{ args.issue_key }}"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        fields = {
          summary = "{{ args.summary }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Updated issue {{ args.issue_key }} successfully."
  }
}

command "delete_issue" {
  title       = "Delete issue"
  summary     = "Delete a Jira issue"
  description = "Permanently delete a Jira issue. Use with caution."
  categories  = ["write", "issues"]

  annotations {
    mode    = "write"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  param "issue_key" {
    type        = "string"
    required    = true
    description = "Issue key to delete"
  }

  param "delete_subtasks" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Also delete subtasks"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://{{ args.domain }}.atlassian.net/rest/api/2/issue/{{ args.issue_key }}"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    query = {
      deleteSubtasks = "{{ args.delete_subtasks }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Deleted issue {{ args.issue_key }}."
  }
}

command "add_comment" {
  title       = "Add comment"
  summary     = "Add a comment to a Jira issue"
  description = "Post a new comment on a Jira issue."
  categories  = ["write", "issues"]

  annotations {
    mode    = "write"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  param "issue_key" {
    type        = "string"
    required    = true
    description = "Issue key to comment on"
  }

  param "body" {
    type        = "string"
    required    = true
    description = "Comment text (plain text)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ args.domain }}.atlassian.net/rest/api/2/issue/{{ args.issue_key }}/comment"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    headers = {
      Accept = "application/json"
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
    output = "Added comment to {{ args.issue_key }} (comment id: {{ result.id }})"
  }
}

command "list_comments" {
  title       = "List comments"
  summary     = "List comments on a Jira issue"
  description = "Retrieve all comments on a Jira issue."
  categories  = ["issues"]

  annotations {
    mode    = "read"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  param "issue_key" {
    type        = "string"
    required    = true
    description = "Issue key"
  }

  param "max_results" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Maximum number of comments to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.domain }}.atlassian.net/rest/api/2/issue/{{ args.issue_key }}/comment"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    query = {
      maxResults = "{{ args.max_results }}"
      orderBy    = "created"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "{{ result.total }} comments on {{ args.issue_key }}."
  }
}

command "transition_issue" {
  title       = "Transition issue"
  summary     = "Change the status of a Jira issue"
  description = "Transition a Jira issue to a new status. Use get_transitions to find available transition IDs."
  categories  = ["write", "issues"]

  annotations {
    mode    = "write"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  param "issue_key" {
    type        = "string"
    required    = true
    description = "Issue key to transition"
  }

  param "transition_id" {
    type        = "string"
    required    = true
    description = "Transition ID (use get_transitions to find available IDs)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ args.domain }}.atlassian.net/rest/api/2/issue/{{ args.issue_key }}/transitions"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        transition = {
          id = "{{ args.transition_id }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Transitioned {{ args.issue_key }} successfully."
  }
}

command "get_transitions" {
  title       = "Get transitions"
  summary     = "Get available status transitions for an issue"
  description = "List the available status transitions for a Jira issue."
  categories  = ["issues"]

  annotations {
    mode    = "read"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  param "issue_key" {
    type        = "string"
    required    = true
    description = "Issue key"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.domain }}.atlassian.net/rest/api/2/issue/{{ args.issue_key }}/transitions"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Available transitions for {{ args.issue_key }}:\n{% for t in result.transitions %}ID: {{ t.id }} -> {{ t.name }} ({{ t.to.name }})\n{% endfor %}"
  }
}

command "assign_issue" {
  title       = "Assign issue"
  summary     = "Assign a Jira issue to a user"
  description = "Set the assignee of a Jira issue. Use search_users to find account IDs."
  categories  = ["write", "issues"]

  annotations {
    mode    = "write"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  param "issue_key" {
    type        = "string"
    required    = true
    description = "Issue key"
  }

  param "account_id" {
    type        = "string"
    required    = true
    description = "Account ID of the assignee (use search_users to find IDs)"
  }

  operation {
    protocol = "http"
    method   = "PUT"
    url      = "https://{{ args.domain }}.atlassian.net/rest/api/2/issue/{{ args.issue_key }}/assignee"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        accountId = "{{ args.account_id }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Assigned {{ args.issue_key }} to account {{ args.account_id }}."
  }
}

command "list_projects" {
  title       = "List projects"
  summary     = "List Jira projects"
  description = "List all accessible Jira projects in the instance."
  categories  = ["projects"]

  annotations {
    mode    = "read"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  param "max_results" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Maximum number of projects to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.domain }}.atlassian.net/rest/api/2/project/search"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    query = {
      maxResults = "{{ args.max_results }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "{{ result.total }} projects found."
  }
}

command "get_myself" {
  title       = "Get current user"
  summary     = "Get details of the authenticated user"
  description = "Retrieve information about the currently authenticated Jira user."
  categories  = ["users"]

  annotations {
    mode    = "read"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.domain }}.atlassian.net/rest/api/2/myself"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "{{ result.displayName }} ({{ result.emailAddress }})\nAccount ID: {{ result.accountId }}\nTimezone: {{ result.timeZone }}"
  }
}

command "search_users" {
  title       = "Search users"
  summary     = "Search for Jira users"
  description = "Search for Jira users by display name or email address."
  categories  = ["users"]

  annotations {
    mode    = "read"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  param "query" {
    type        = "string"
    required    = true
    description = "Search string (matches display name and email)"
  }

  param "max_results" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Maximum number of results"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.domain }}.atlassian.net/rest/api/2/user/search"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    query = {
      query      = "{{ args.query }}"
      maxResults = "{{ args.max_results }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "{{ result | length }} users found."
  }
}

command "list_sprints" {
  title       = "List sprints"
  summary     = "List sprints for a Jira board"
  description = "List sprints associated with an agile board. Use list_boards to find board IDs."
  categories  = ["agile"]

  annotations {
    mode    = "read"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  param "board_id" {
    type        = "integer"
    required    = true
    description = "Board ID (use list_boards to find IDs)"
  }

  param "state" {
    type        = "string"
    required    = false
    default     = ""
    description = "Filter by state: 'future', 'active', 'closed' (comma-separated)"
  }

  param "max_results" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Maximum number of sprints to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.domain }}.atlassian.net/rest/agile/1.0/board/{{ args.board_id }}/sprint"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    query = {
      state      = "{{ args.state }}"
      maxResults = "{{ args.max_results }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Sprints for board {{ args.board_id }}:\n{% for s in result.values %}[{{ s.state }}] {{ s.name }} (id: {{ s.id }})\n{% endfor %}"
  }
}

command "list_boards" {
  title       = "List boards"
  summary     = "List Jira agile boards"
  description = "List agile boards accessible to the current user."
  categories  = ["agile"]

  annotations {
    mode    = "read"
    secrets = ["jira.email", "jira.api_token"]
  }

  param "domain" {
    type        = "string"
    required    = true
    description = "Atlassian subdomain"
  }

  param "type" {
    type        = "string"
    required    = false
    default     = ""
    description = "Board type: 'scrum' or 'kanban'"
  }

  param "project_key" {
    type        = "string"
    required    = false
    default     = ""
    description = "Filter by project key"
  }

  param "max_results" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Maximum number of boards to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.domain }}.atlassian.net/rest/agile/1.0/board"

    auth {
      kind            = "basic"
      username        = "{{ secrets.jira_email }}"
      password_secret = "jira.api_token"
    }

    query = {
      type           = "{{ args.type }}"
      projectKeyOrId = "{{ args.project_key }}"
      maxResults     = "{{ args.max_results }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "{{ result.total }} boards found."
  }
}
