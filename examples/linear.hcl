version = 1
provider = "linear"
categories = ["project-management", "issue-tracking"]

command "viewer" {
  title       = "Current user"
  summary     = "Get the authenticated user's profile"
  description = "Fetch the display name, email, and active status of the currently authenticated Linear user."
  categories  = ["profile"]

  annotations {
    mode    = "read"
    secrets = ["linear.api_key"]
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        {
          viewer {
            id
            name
            email
            active
            admin
            displayName
            createdAt
          }
        }
      EOT
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/viewer"
    }
    output = "{{ result.displayName }} ({{ result.email }}) — active: {{ result.active }}"
  }
}

command "list_issues" {
  title       = "List issues"
  summary     = "List recent issues with optional filtering"
  description = "Fetch recent issues from Linear, ordered by last updated. Optionally filter by team."
  categories  = ["issues"]

  annotations {
    mode    = "read"
    secrets = ["linear.api_key"]
  }

  param "count" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of issues to fetch"
  }

  param "team_key" {
    type        = "string"
    required    = false
    default     = ""
    description = "Team key to filter by (e.g. 'ENG'). Leave empty for all teams."
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        query ListIssues($count: Int!, $teamKey: String) {
          issues(
            first: $count
            orderBy: updatedAt
            filter: { team: { key: { eq: $teamKey } } }
          ) {
            nodes {
              identifier
              title
              priority
              state { name }
              assignee { displayName }
              updatedAt
            }
          }
        }
      EOT
      variables = {
        count   = "{{ args.count }}"
        teamKey = "{{ args.team_key if args.team_key else None }}"
      }
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/issues"
    }
    output = "{{ result.nodes | length }} issues returned"
  }
}

command "get_issue" {
  title       = "Get issue"
  summary     = "Get a single issue by its identifier"
  description = "Fetch full details of a Linear issue by its identifier (e.g. 'ENG-123')."
  categories  = ["issues"]

  annotations {
    mode    = "read"
    secrets = ["linear.api_key"]
  }

  param "issue_id" {
    type        = "string"
    required    = true
    description = "Issue identifier (e.g. 'ENG-123')"
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        query GetIssue($id: String!) {
          issueSearch(query: $id, first: 1) {
            nodes {
              identifier
              title
              description
              priority
              estimate
              state { name }
              assignee { displayName }
              team { name key }
              project { name }
              labels { nodes { name } }
              createdAt
              updatedAt
            }
          }
        }
      EOT
      variables = {
        id = "{{ args.issue_id }}"
      }
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/issueSearch/nodes/0"
    }
    output = "{{ result.identifier }}: {{ result.title }} [{{ result.state.name }}] — assigned to {{ result.assignee.displayName if result.assignee else 'unassigned' }}"
  }
}

command "create_issue" {
  title       = "Create issue"
  summary     = "Create a new issue in a team"
  description = "Create a new Linear issue in the specified team with a title and optional description."
  categories  = ["issues"]

  annotations {
    mode    = "write"
    secrets = ["linear.api_key"]
  }

  param "team_id" {
    type        = "string"
    required    = true
    description = "Team ID to create the issue in"
  }

  param "title" {
    type        = "string"
    required    = true
    description = "Issue title"
  }

  param "description" {
    type        = "string"
    required    = false
    default     = ""
    description = "Issue description (Markdown)"
  }

  param "priority" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Priority: 0=none, 1=urgent, 2=high, 3=medium, 4=low"
  }

  param "assignee_id" {
    type        = "string"
    required    = false
    default     = ""
    description = "User ID to assign the issue to"
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        mutation CreateIssue($input: IssueCreateInput!) {
          issueCreate(input: $input) {
            success
            issue {
              identifier
              title
              url
            }
          }
        }
      EOT
      variables = {
        input = {
          teamId      = "{{ args.team_id }}"
          title       = "{{ args.title }}"
          description = "{{ args.description if args.description else None }}"
          priority    = "{{ args.priority }}"
          assigneeId  = "{{ args.assignee_id if args.assignee_id else None }}"
        }
      }
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/issueCreate"
    }
    output = "{{ 'Created ' ~ result.issue.identifier ~ ': ' ~ result.issue.title ~ ' — ' ~ result.issue.url if result.success else 'Failed to create issue' }}"
  }
}

command "update_issue" {
  title       = "Update issue"
  summary     = "Update an existing issue"
  description = "Update fields on an existing Linear issue such as title, state, priority, or assignee."
  categories  = ["issues"]

  annotations {
    mode    = "write"
    secrets = ["linear.api_key"]
  }

  param "issue_id" {
    type        = "string"
    required    = true
    description = "Issue UUID to update"
  }

  param "title" {
    type        = "string"
    required    = false
    default     = ""
    description = "New issue title"
  }

  param "state_id" {
    type        = "string"
    required    = false
    default     = ""
    description = "Workflow state ID to transition to"
  }

  param "priority" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Priority: 0=none, 1=urgent, 2=high, 3=medium, 4=low"
  }

  param "assignee_id" {
    type        = "string"
    required    = false
    default     = ""
    description = "User ID to assign the issue to"
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        mutation UpdateIssue($id: String!, $input: IssueUpdateInput!) {
          issueUpdate(id: $id, input: $input) {
            success
            issue {
              identifier
              title
              state { name }
              url
            }
          }
        }
      EOT
      variables = {
        id = "{{ args.issue_id }}"
        input = {
          title      = "{{ args.title if args.title else None }}"
          stateId    = "{{ args.state_id if args.state_id else None }}"
          priority   = "{{ args.priority if args.priority else None }}"
          assigneeId = "{{ args.assignee_id if args.assignee_id else None }}"
        }
      }
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/issueUpdate"
    }
    output = "{{ 'Updated ' ~ result.issue.identifier ~ ': ' ~ result.issue.title ~ ' [' ~ result.issue.state.name ~ ']' if result.success else 'Failed to update issue' }}"
  }
}

command "delete_issue" {
  title       = "Delete issue"
  summary     = "Archive/delete an issue"
  description = "Permanently delete a Linear issue. This action cannot be undone."
  categories  = ["issues"]

  annotations {
    mode    = "write"
    secrets = ["linear.api_key"]
  }

  param "issue_id" {
    type        = "string"
    required    = true
    description = "Issue UUID to delete"
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        mutation DeleteIssue($id: String!) {
          issueDelete(id: $id) {
            success
          }
        }
      EOT
      variables = {
        id = "{{ args.issue_id }}"
      }
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/issueDelete"
    }
    output = "{{ 'Issue deleted successfully' if result.success else 'Failed to delete issue' }}"
  }
}

command "search_issues" {
  title       = "Search issues"
  summary     = "Search issues by text query"
  description = "Search Linear issues by a free-text query string across titles and descriptions."
  categories  = ["search", "issues"]

  annotations {
    mode    = "read"
    secrets = ["linear.api_key"]
  }

  param "query" {
    type        = "string"
    required    = true
    description = "Search query text"
  }

  param "count" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of results to return"
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        query SearchIssues($query: String!, $count: Int!) {
          issueSearch(query: $query, first: $count) {
            nodes {
              identifier
              title
              priority
              state { name }
              assignee { displayName }
              team { key }
              updatedAt
            }
          }
        }
      EOT
      variables = {
        query = "{{ args.query }}"
        count = "{{ args.count }}"
      }
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/issueSearch"
    }
    output = "{{ result.nodes | length }} issues found"
  }
}

command "list_teams" {
  title       = "List teams"
  summary     = "List all teams in the workspace"
  description = "Fetch all teams in the Linear workspace with their keys, names, and member counts."
  categories  = ["teams"]

  annotations {
    mode    = "read"
    secrets = ["linear.api_key"]
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        {
          teams {
            nodes {
              id
              name
              key
              description
              members {
                nodes { displayName }
              }
            }
          }
        }
      EOT
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/teams"
    }
    output = "{{ result.nodes | length }} teams found"
  }
}

command "list_projects" {
  title       = "List projects"
  summary     = "List projects in the workspace"
  description = "Fetch projects from the Linear workspace with their status and progress."
  categories  = ["projects"]

  annotations {
    mode    = "read"
    secrets = ["linear.api_key"]
  }

  param "count" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of projects to fetch"
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        query ListProjects($count: Int!) {
          projects(first: $count, orderBy: updatedAt) {
            nodes {
              id
              name
              description
              state
              progress
              targetDate
              lead { displayName }
              teams { nodes { key } }
            }
          }
        }
      EOT
      variables = {
        count = "{{ args.count }}"
      }
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/projects"
    }
    output = "{{ result.nodes | length }} projects returned"
  }
}

command "create_comment" {
  title       = "Create comment"
  summary     = "Add a comment to an issue"
  description = "Create a new comment on a Linear issue."
  categories  = ["issues", "comments"]

  annotations {
    mode    = "write"
    secrets = ["linear.api_key"]
  }

  param "issue_id" {
    type        = "string"
    required    = true
    description = "Issue UUID to comment on"
  }

  param "body" {
    type        = "string"
    required    = true
    description = "Comment body (Markdown)"
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        mutation CreateComment($input: CommentCreateInput!) {
          commentCreate(input: $input) {
            success
            comment {
              id
              body
              createdAt
              user { displayName }
            }
          }
        }
      EOT
      variables = {
        input = {
          issueId = "{{ args.issue_id }}"
          body    = "{{ args.body }}"
        }
      }
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/commentCreate"
    }
    output = "{{ 'Comment added by ' ~ result.comment.user.displayName if result.success else 'Failed to create comment' }}"
  }
}

command "list_cycles" {
  title       = "List cycles"
  summary     = "List cycles for a team"
  description = "Fetch cycles (sprints) for a given team, including their progress and dates."
  categories  = ["cycles"]

  annotations {
    mode    = "read"
    secrets = ["linear.api_key"]
  }

  param "team_id" {
    type        = "string"
    required    = true
    description = "Team UUID to list cycles for"
  }

  param "count" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of cycles to fetch"
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        query ListCycles($teamId: String!, $count: Int!) {
          team(id: $teamId) {
            cycles(first: $count, orderBy: createdAt) {
              nodes {
                id
                number
                name
                startsAt
                endsAt
                progress
                completedScopeHistory
                issueCountHistory
              }
            }
          }
        }
      EOT
      variables = {
        teamId = "{{ args.team_id }}"
        count  = "{{ args.count }}"
      }
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/team/cycles"
    }
    output = "{{ result.nodes | length }} cycles returned"
  }
}

command "list_labels" {
  title       = "List labels"
  summary     = "List all issue labels"
  description = "Fetch all issue labels available in the Linear workspace."
  categories  = ["labels"]

  annotations {
    mode    = "read"
    secrets = ["linear.api_key"]
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        {
          issueLabels(first: 100) {
            nodes {
              id
              name
              color
              parent { name }
            }
          }
        }
      EOT
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/issueLabels"
    }
    output = "{{ result.nodes | length }} labels found"
  }
}

command "list_workflow_states" {
  title       = "List workflow states"
  summary     = "List workflow states for a team"
  description = "Fetch all workflow states (e.g. Backlog, Todo, In Progress, Done) for a given team."
  categories  = ["teams", "workflow"]

  annotations {
    mode    = "read"
    secrets = ["linear.api_key"]
  }

  param "team_id" {
    type        = "string"
    required    = true
    description = "Team UUID to list workflow states for"
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        query ListWorkflowStates($teamId: String!) {
          team(id: $teamId) {
            states {
              nodes {
                id
                name
                type
                color
                position
              }
            }
          }
        }
      EOT
      variables = {
        teamId = "{{ args.team_id }}"
      }
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/team/states"
    }
    output = "{{ result.nodes | length }} workflow states"
  }
}

command "add_label" {
  title       = "Add label to issue"
  summary     = "Add a label to an existing issue"
  description = "Attach an existing label to a Linear issue by adding it to the issue's label set."
  categories  = ["issues", "labels"]

  annotations {
    mode    = "write"
    secrets = ["linear.api_key"]
  }

  param "issue_id" {
    type        = "string"
    required    = true
    description = "Issue UUID to label"
  }

  param "label_id" {
    type        = "string"
    required    = true
    description = "Label UUID to add"
  }

  operation {
    protocol = "graphql"
    url      = "https://api.linear.app/graphql"

    auth {
      kind   = "bearer"
      secret = "linear.api_key"
    }

    graphql {
      query = <<-EOT
        mutation AddLabel($issueId: String!, $labelId: String!) {
          issueAddLabel(id: $issueId, labelId: $labelId) {
            success
            issue {
              identifier
              title
              labels { nodes { name } }
            }
          }
        }
      EOT
      variables = {
        issueId = "{{ args.issue_id }}"
        labelId = "{{ args.label_id }}"
      }
    }
  }

  result {
    decode = "json"
    extract {
      json_pointer = "/data/issueAddLabel"
    }
    output = "{{ 'Label added to ' ~ result.issue.identifier if result.success else 'Failed to add label' }}"
  }
}
