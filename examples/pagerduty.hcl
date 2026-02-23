version = 1
provider = "pagerduty"
categories = ["incident-management", "on-call", "monitoring"]

command "list_incidents" {
  title       = "List Incidents"
  summary     = "List and filter PagerDuty incidents"
  description = "Retrieve incidents sorted by creation date. Set your pagerduty.api_key secret to 'Token token=YOUR_API_KEY'."
  categories  = ["incidents"]

  annotations {
    mode    = "read"
    secrets = ["pagerduty.api_key"]
  }

  param "date_range" {
    type        = "string"
    required    = false
    default     = "all"
    description = "Set to 'all' for all incidents or omit for last 6 months"
  }

  param "sort_by" {
    type        = "string"
    required    = false
    default     = "created_at:desc"
    description = "Sort order: created_at:desc, created_at:asc, resolved_at:desc, resolved_at:asc"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 25
    description = "Max results per page (max 100)"
  }

  param "offset" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Pagination offset"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.pagerduty.com/incidents"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    query = {
      date_range = "{{ args.date_range }}"
      sort_by    = "{{ args.sort_by }}"
      limit      = "{{ args.limit }}"
      offset     = "{{ args.offset }}"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.incidents | length }} incidents.{% for i in result.incidents %}\n- [{{ i.status | upper }}] #{{ i.incident_number }}: {{ i.title }} ({{ i.service.summary }}){% endfor %}"
  }
}

command "get_incident" {
  title       = "Get Incident"
  summary     = "Get details of a specific incident"
  description = "Retrieve full details of a PagerDuty incident by its ID."
  categories  = ["incidents"]

  annotations {
    mode    = "read"
    secrets = ["pagerduty.api_key"]
  }

  param "id" {
    type        = "string"
    required    = true
    description = "Incident ID (e.g. PABC123)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.pagerduty.com/incidents/{{ args.id }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
    }
  }

  result {
    decode = "json"
    output = "Incident #{{ result.incident.incident_number }}: {{ result.incident.title }}\nStatus: {{ result.incident.status }} | Urgency: {{ result.incident.urgency }}\nService: {{ result.incident.service.summary }}\nEscalation Policy: {{ result.incident.escalation_policy.summary }}\nCreated: {{ result.incident.created_at }}\nURL: {{ result.incident.html_url }}"
  }
}

command "create_incident" {
  title       = "Create Incident"
  summary     = "Create a new PagerDuty incident"
  description = "Create an incident on a specified service. Requires your PagerDuty user email in the from_email parameter."
  categories  = ["incidents"]

  annotations {
    mode    = "write"
    secrets = ["pagerduty.api_key"]
  }

  param "title" {
    type        = "string"
    required    = true
    description = "Incident title"
  }

  param "service_id" {
    type        = "string"
    required    = true
    description = "ID of the service to create the incident on"
  }

  param "from_email" {
    type        = "string"
    required    = true
    description = "Email address of the PagerDuty user creating the incident"
  }

  param "urgency" {
    type        = "string"
    required    = false
    default     = "high"
    description = "Incident urgency: high or low"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.pagerduty.com/incidents"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
      From   = "{{ args.from_email }}"
    }

    body {
      kind = "json"
      value = {
        incident = {
          type    = "incident"
          title   = "{{ args.title }}"
          urgency = "{{ args.urgency }}"
          service = {
            id   = "{{ args.service_id }}"
            type = "service_reference"
          }
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Created incident #{{ result.incident.incident_number }}: {{ result.incident.title }}\nID: {{ result.incident.id }} | Status: {{ result.incident.status }}\nService: {{ result.incident.service.summary }}\nURL: {{ result.incident.html_url }}"
  }
}

command "update_incident" {
  title       = "Update Incident"
  summary     = "Acknowledge or resolve an incident"
  description = "Update an incident's status to acknowledged or resolved."
  categories  = ["incidents"]

  annotations {
    mode    = "write"
    secrets = ["pagerduty.api_key"]
  }

  param "id" {
    type        = "string"
    required    = true
    description = "Incident ID"
  }

  param "status" {
    type        = "string"
    required    = true
    description = "New status: acknowledged or resolved"
  }

  param "from_email" {
    type        = "string"
    required    = true
    description = "Email address of the PagerDuty user making the update"
  }

  operation {
    protocol = "http"
    method   = "PUT"
    url      = "https://api.pagerduty.com/incidents/{{ args.id }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
      From   = "{{ args.from_email }}"
    }

    body {
      kind = "json"
      value = {
        incident = {
          type   = "incident"
          status = "{{ args.status }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Updated incident #{{ result.incident.incident_number }}: {{ result.incident.title }}\nStatus: {{ result.incident.status }} | Urgency: {{ result.incident.urgency }}\nURL: {{ result.incident.html_url }}"
  }
}

command "add_incident_note" {
  title       = "Add Incident Note"
  summary     = "Add a note to an incident"
  description = "Post a note to the timeline of an existing incident."
  categories  = ["incidents"]

  annotations {
    mode    = "write"
    secrets = ["pagerduty.api_key"]
  }

  param "id" {
    type        = "string"
    required    = true
    description = "Incident ID"
  }

  param "content" {
    type        = "string"
    required    = true
    description = "Note text"
  }

  param "from_email" {
    type        = "string"
    required    = true
    description = "Email address of the PagerDuty user adding the note"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.pagerduty.com/incidents/{{ args.id }}/notes"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
      From   = "{{ args.from_email }}"
    }

    body {
      kind = "json"
      value = {
        note = {
          content = "{{ args.content }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Added note to incident {{ args.id }}\nBy: {{ result.note.user.summary }} at {{ result.note.created_at }}"
  }
}

command "list_incident_alerts" {
  title       = "List Incident Alerts"
  summary     = "List alerts grouped under an incident"
  description = "Retrieve alerts associated with a specific incident."
  categories  = ["incidents"]

  annotations {
    mode    = "read"
    secrets = ["pagerduty.api_key"]
  }

  param "id" {
    type        = "string"
    required    = true
    description = "Incident ID"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 25
    description = "Max results per page (max 100)"
  }

  param "offset" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Pagination offset"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.pagerduty.com/incidents/{{ args.id }}/alerts"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    query = {
      limit  = "{{ args.limit }}"
      offset = "{{ args.offset }}"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.alerts | length }} alerts for incident {{ args.id }}.{% for a in result.alerts %}\n- [{{ a.status | upper }}] {{ a.summary }} (severity: {{ a.severity }}){% endfor %}"
  }
}

command "list_incident_notes" {
  title       = "List Incident Notes"
  summary     = "List notes on an incident"
  description = "Retrieve all notes posted to an incident's timeline."
  categories  = ["incidents"]

  annotations {
    mode    = "read"
    secrets = ["pagerduty.api_key"]
  }

  param "id" {
    type        = "string"
    required    = true
    description = "Incident ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.pagerduty.com/incidents/{{ args.id }}/notes"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
    }
  }

  result {
    decode = "json"
    output = "{{ result.notes | length }} notes for incident {{ args.id }}.{% for n in result.notes %}\n- [{{ n.created_at }}] {{ n.user.summary }}: {{ n.content }}{% endfor %}"
  }
}

command "list_services" {
  title       = "List Services"
  summary     = "List PagerDuty services"
  description = "Retrieve services, optionally filtered by name."
  categories  = ["services"]

  annotations {
    mode    = "read"
    secrets = ["pagerduty.api_key"]
  }

  param "query" {
    type        = "string"
    required    = false
    default     = ""
    description = "Filter by service name"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 25
    description = "Max results per page (max 100)"
  }

  param "offset" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Pagination offset"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.pagerduty.com/services"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    query = {
      query  = "{{ args.query }}"
      limit  = "{{ args.limit }}"
      offset = "{{ args.offset }}"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.services | length }} services.{% for s in result.services %}\n- [{{ s.status | upper }}] {{ s.name }} ({{ s.id }}) — {{ s.escalation_policy.summary }}{% endfor %}"
  }
}

command "get_service" {
  title       = "Get Service"
  summary     = "Get details of a specific service"
  description = "Retrieve full details of a PagerDuty service by its ID."
  categories  = ["services"]

  annotations {
    mode    = "read"
    secrets = ["pagerduty.api_key"]
  }

  param "id" {
    type        = "string"
    required    = true
    description = "Service ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.pagerduty.com/services/{{ args.id }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
    }
  }

  result {
    decode = "json"
    output = "Service: {{ result.service.name }} ({{ result.service.id }})\nStatus: {{ result.service.status }}\nDescription: {{ result.service.description }}\nEscalation Policy: {{ result.service.escalation_policy.summary }}\nAlert Creation: {{ result.service.alert_creation }}\nURL: {{ result.service.html_url }}"
  }
}

command "list_users" {
  title       = "List Users"
  summary     = "List PagerDuty users"
  description = "Retrieve users, optionally filtered by name or email."
  categories  = ["users"]

  annotations {
    mode    = "read"
    secrets = ["pagerduty.api_key"]
  }

  param "query" {
    type        = "string"
    required    = false
    default     = ""
    description = "Filter by name or email"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 25
    description = "Max results per page (max 100)"
  }

  param "offset" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Pagination offset"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.pagerduty.com/users"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    query = {
      query  = "{{ args.query }}"
      limit  = "{{ args.limit }}"
      offset = "{{ args.offset }}"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.users | length }} users.{% for u in result.users %}\n- {{ u.name }} <{{ u.email }}> ({{ u.id }}) — {{ u.role }}{% endfor %}"
  }
}

command "get_user" {
  title       = "Get User"
  summary     = "Get details of a specific user"
  description = "Retrieve full details of a PagerDuty user by their ID."
  categories  = ["users"]

  annotations {
    mode    = "read"
    secrets = ["pagerduty.api_key"]
  }

  param "id" {
    type        = "string"
    required    = true
    description = "User ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.pagerduty.com/users/{{ args.id }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
    }
  }

  result {
    decode = "json"
    output = "User: {{ result.user.name }} ({{ result.user.id }})\nEmail: {{ result.user.email }}\nRole: {{ result.user.role }}\nTimezone: {{ result.user.time_zone }}\nURL: {{ result.user.html_url }}"
  }
}

command "list_oncalls" {
  title       = "List On-Calls"
  summary     = "List current on-call entries"
  description = "Retrieve who is currently on-call, optionally filtered by schedule or escalation policy."
  categories  = ["on-call"]

  annotations {
    mode    = "read"
    secrets = ["pagerduty.api_key"]
  }

  param "schedule_ids" {
    type        = "string"
    required    = false
    default     = ""
    description = "Comma-separated schedule IDs to filter by"
  }

  param "escalation_policy_ids" {
    type        = "string"
    required    = false
    default     = ""
    description = "Comma-separated escalation policy IDs to filter by"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 25
    description = "Max results per page (max 100)"
  }

  param "offset" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Pagination offset"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.pagerduty.com/oncalls"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    query = {
      "schedule_ids[]"          = "{{ args.schedule_ids }}"
      "escalation_policy_ids[]" = "{{ args.escalation_policy_ids }}"
      limit                     = "{{ args.limit }}"
      offset                    = "{{ args.offset }}"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
    }
  }

  result {
    decode = "json"
    output = "{{ result.oncalls | length }} on-call entries.{% for o in result.oncalls %}\n- {{ o.user.summary }} — Level {{ o.escalation_level }}, Policy: {{ o.escalation_policy.summary }} ({{ o.start }} to {{ o.end }}){% endfor %}"
  }
}

command "list_escalation_policies" {
  title       = "List Escalation Policies"
  summary     = "List PagerDuty escalation policies"
  description = "Retrieve escalation policies, optionally filtered by name."
  categories  = ["on-call"]

  annotations {
    mode    = "read"
    secrets = ["pagerduty.api_key"]
  }

  param "query" {
    type        = "string"
    required    = false
    default     = ""
    description = "Filter by policy name"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 25
    description = "Max results per page (max 100)"
  }

  param "offset" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Pagination offset"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.pagerduty.com/escalation_policies"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    query = {
      query  = "{{ args.query }}"
      limit  = "{{ args.limit }}"
      offset = "{{ args.offset }}"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.escalation_policies | length }} escalation policies.{% for ep in result.escalation_policies %}\n- {{ ep.name }} ({{ ep.id }}) — {{ ep.num_loops }} loops{% endfor %}"
  }
}

command "list_schedules" {
  title       = "List Schedules"
  summary     = "List PagerDuty on-call schedules"
  description = "Retrieve on-call schedules, optionally filtered by name."
  categories  = ["on-call"]

  annotations {
    mode    = "read"
    secrets = ["pagerduty.api_key"]
  }

  param "query" {
    type        = "string"
    required    = false
    default     = ""
    description = "Filter by schedule name"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 25
    description = "Max results per page (max 100)"
  }

  param "offset" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Pagination offset"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.pagerduty.com/schedules"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    query = {
      query  = "{{ args.query }}"
      limit  = "{{ args.limit }}"
      offset = "{{ args.offset }}"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.schedules | length }} schedules.{% for s in result.schedules %}\n- {{ s.name }} ({{ s.id }}) — {{ s.time_zone }}{% endfor %}"
  }
}

command "list_teams" {
  title       = "List Teams"
  summary     = "List PagerDuty teams"
  description = "Retrieve teams, optionally filtered by name."
  categories  = ["teams"]

  annotations {
    mode    = "read"
    secrets = ["pagerduty.api_key"]
  }

  param "query" {
    type        = "string"
    required    = false
    default     = ""
    description = "Filter by team name"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 25
    description = "Max results per page (max 100)"
  }

  param "offset" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Pagination offset"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.pagerduty.com/teams"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "Authorization"
      secret   = "pagerduty.api_key"
    }

    query = {
      query  = "{{ args.query }}"
      limit  = "{{ args.limit }}"
      offset = "{{ args.offset }}"
    }

    headers = {
      Accept = "application/vnd.pagerduty+json;version=2"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.teams | length }} teams.{% for t in result.teams %}\n- {{ t.name }} ({{ t.id }}){% if t.description %} — {{ t.description }}{% endif %}{% endfor %}"
  }
}
