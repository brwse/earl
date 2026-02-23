version = 1
provider = "datadog"
categories = ["monitoring", "observability", "infrastructure"]

command "list_monitors" {
  title       = "List monitors"
  summary     = "List all Datadog monitors"
  description = "Retrieve all monitors from your Datadog account, optionally filtered by tags."
  categories  = ["monitors"]

  annotations {
    mode    = "read"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "tags" {
    type        = "string"
    required    = false
    default     = ""
    description = "Comma-separated list of tags to filter by"
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Number of monitors per page (max 1000)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.datadoghq.com/api/v1/monitor"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    query = {
      tags      = "{{ args.tags }}"
      page_size = "{{ args.page_size }}"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result | length }} monitors."
  }
}

command "get_monitor" {
  title       = "Get monitor"
  summary     = "Get a monitor by ID"
  description = "Retrieve details for a specific Datadog monitor by its ID."
  categories  = ["monitors"]

  annotations {
    mode    = "read"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "monitor_id" {
    type        = "integer"
    required    = true
    description = "The monitor ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.datadoghq.com/api/v1/monitor/{{ args.monitor_id }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }
  }

  result {
    decode = "json"
    output = "Monitor [{{ result.id }}]: {{ result.name }}\nType: {{ result.type }}\nStatus: {{ result.overall_state }}\nQuery: {{ result.query }}"
  }
}

command "create_monitor" {
  title       = "Create monitor"
  summary     = "Create a new Datadog monitor"
  description = "Create a new monitor with the specified type, query, and notification message."
  categories  = ["monitors"]

  annotations {
    mode    = "write"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Monitor name"
  }

  param "type" {
    type        = "string"
    required    = true
    description = "Monitor type (e.g. 'metric alert', 'log alert', 'query alert')"
  }

  param "query" {
    type        = "string"
    required    = true
    description = "The monitor query (e.g. 'avg(last_5m):avg:system.cpu.user{*} > 90')"
  }

  param "message" {
    type        = "string"
    required    = true
    description = "Notification message body, supports @-mentions and markdown"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.datadoghq.com/api/v1/monitor"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }

    body {
      kind = "json"
      value = {
        name    = "{{ args.name }}"
        type    = "{{ args.type }}"
        query   = "{{ args.query }}"
        message = "{{ args.message }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created monitor [{{ result.id }}]: {{ result.name }} ({{ result.type }})"
  }
}

command "delete_monitor" {
  title       = "Delete monitor"
  summary     = "Delete a monitor by ID"
  description = "Permanently delete a Datadog monitor."
  categories  = ["monitors"]

  annotations {
    mode    = "write"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "monitor_id" {
    type        = "integer"
    required    = true
    description = "The monitor ID to delete"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.datadoghq.com/api/v1/monitor/{{ args.monitor_id }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }
  }

  result {
    decode = "json"
    output = "Deleted monitor {{ args.monitor_id }}."
  }
}

command "search_logs" {
  title       = "Search logs"
  summary     = "Search log events with a query"
  description = "Search Datadog log events using a log search query with time range filtering."
  categories  = ["logs"]

  annotations {
    mode    = "read"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "query" {
    type        = "string"
    required    = true
    description = "Datadog log search query (e.g. 'service:frontend status:error')"
  }

  param "from" {
    type        = "string"
    required    = false
    default     = "now-15m"
    description = "Start time as ISO 8601 or relative (e.g. 'now-1h')"
  }

  param "to" {
    type        = "string"
    required    = false
    default     = "now"
    description = "End time as ISO 8601 or relative"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Maximum number of log entries to return (max 1000)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.datadoghq.com/api/v2/logs/events/search"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }

    body {
      kind = "json"
      value = {
        filter = {
          query = "{{ args.query }}"
          from  = "{{ args.from }}"
          to    = "{{ args.to }}"
        }
        page = {
          limit = "{{ args.limit }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.data | length }} log entries."
  }
}

command "create_event" {
  title       = "Create event"
  summary     = "Post an event to the Datadog event stream"
  description = "Submit a custom event to the Datadog event stream for tracking deployments, alerts, or other notable occurrences."
  categories  = ["events"]

  annotations {
    mode    = "write"
    secrets = ["datadog.api_key"]
  }

  param "title" {
    type        = "string"
    required    = true
    description = "Event title"
  }

  param "text" {
    type        = "string"
    required    = true
    description = "Event body text (max 4000 characters)"
  }

  param "alert_type" {
    type        = "string"
    required    = false
    default     = "info"
    description = "Alert type: error, warning, info, or success"
  }

  param "priority" {
    type        = "string"
    required    = false
    default     = "normal"
    description = "Event priority: normal or low"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.datadoghq.com/api/v1/events"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    body {
      kind = "json"
      value = {
        title      = "{{ args.title }}"
        text       = "{{ args.text }}"
        alert_type = "{{ args.alert_type }}"
        priority   = "{{ args.priority }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created event: {{ result.event.title }} ({{ result.status }})"
  }
}

command "list_events" {
  title       = "List events"
  summary     = "List events within a time range"
  description = "Retrieve events from the Datadog event stream within a specified time range."
  categories  = ["events"]

  annotations {
    mode    = "read"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "start" {
    type        = "integer"
    required    = true
    description = "POSIX timestamp for the start of the time range"
  }

  param "end" {
    type        = "integer"
    required    = true
    description = "POSIX timestamp for the end of the time range"
  }

  param "priority" {
    type        = "string"
    required    = false
    default     = ""
    description = "Filter by priority: normal or low"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.datadoghq.com/api/v1/events"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    query = {
      start    = "{{ args.start }}"
      end      = "{{ args.end }}"
      priority = "{{ args.priority }}"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.events | length }} events."
  }
}

command "query_metrics" {
  title       = "Query metrics"
  summary     = "Query timeseries metric data"
  description = "Query Datadog metric timeseries data for a given time range using a metric query expression."
  categories  = ["metrics"]

  annotations {
    mode    = "read"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "from" {
    type        = "integer"
    required    = true
    description = "POSIX timestamp for the start of the query window"
  }

  param "to" {
    type        = "integer"
    required    = true
    description = "POSIX timestamp for the end of the query window"
  }

  param "query" {
    type        = "string"
    required    = true
    description = "Metric query expression (e.g. 'avg:system.cpu.user{*}')"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.datadoghq.com/api/v1/query"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    query = {
      from  = "{{ args.from }}"
      to    = "{{ args.to }}"
      query = "{{ args.query }}"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }
  }

  result {
    decode = "json"
    output = "Query status: {{ result.status }}\n{{ result.series | length }} series returned."
  }
}

command "list_dashboards" {
  title       = "List dashboards"
  summary     = "List all Datadog dashboards"
  description = "Retrieve all dashboards from your Datadog account."
  categories  = ["dashboards"]

  annotations {
    mode    = "read"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "count" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Number of dashboards to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.datadoghq.com/api/v1/dashboard"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    query = {
      count = "{{ args.count }}"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.dashboards | length }} dashboards."
  }
}

command "get_dashboard" {
  title       = "Get dashboard"
  summary     = "Get a dashboard by ID"
  description = "Retrieve details for a specific Datadog dashboard."
  categories  = ["dashboards"]

  annotations {
    mode    = "read"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "dashboard_id" {
    type        = "string"
    required    = true
    description = "The dashboard ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.datadoghq.com/api/v1/dashboard/{{ args.dashboard_id }}"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }
  }

  result {
    decode = "json"
    output = "Dashboard [{{ result.id }}]: {{ result.title }}\nLayout: {{ result.layout_type }}\nWidgets: {{ result.widgets | length }}\nURL: {{ result.url }}"
  }
}

command "list_hosts" {
  title       = "List hosts"
  summary     = "List infrastructure hosts"
  description = "Retrieve a list of hosts reporting to your Datadog account, optionally filtered by name or tag."
  categories  = ["infrastructure"]

  annotations {
    mode    = "read"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "filter" {
    type        = "string"
    required    = false
    default     = ""
    description = "Filter hosts by name, alias, or tag"
  }

  param "count" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Maximum number of hosts to return (max 1000)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.datadoghq.com/api/v1/hosts"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    query = {
      filter = "{{ args.filter }}"
      count  = "{{ args.count }}"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }
  }

  result {
    decode = "json"
    output = "{{ result.total_matching }} hosts found (showing {{ result.total_returned }})."
  }
}

command "mute_host" {
  title       = "Mute host"
  summary     = "Mute a host to suppress alerts"
  description = "Mute monitoring notifications for a specific host, optionally until a specified time."
  categories  = ["infrastructure"]

  annotations {
    mode    = "write"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "host_name" {
    type        = "string"
    required    = true
    description = "The hostname to mute"
  }

  param "message" {
    type        = "string"
    required    = false
    default     = ""
    description = "Reason for muting the host"
  }

  param "end" {
    type        = "integer"
    required    = false
    default     = 0
    description = "POSIX timestamp when the mute should expire (0 for indefinite)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.datadoghq.com/api/v1/host/{{ args.host_name }}/mute"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }

    body {
      kind = "json"
      value = {
        message  = "{{ args.message }}"
        end      = "{{ args.end }}"
        override = true
      }
    }
  }

  result {
    decode = "json"
    output = "Muted host {{ result.hostname }}."
  }
}

command "list_incidents" {
  title       = "List incidents"
  summary     = "List Datadog incidents"
  description = "Retrieve a paginated list of incidents from your Datadog account."
  categories  = ["incidents"]

  annotations {
    mode    = "read"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of incidents per page"
  }

  param "page_offset" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Pagination offset"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.datadoghq.com/api/v2/incidents"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    query = {
      "page[size]"   = "{{ args.page_size }}"
      "page[offset]" = "{{ args.page_offset }}"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.data | length }} incidents."
  }
}

command "create_incident" {
  title       = "Create incident"
  summary     = "Create a new Datadog incident"
  description = "Declare a new incident in Datadog with a title and customer impact status."
  categories  = ["incidents"]

  annotations {
    mode    = "write"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "title" {
    type        = "string"
    required    = true
    description = "Incident title"
  }

  param "customer_impacted" {
    type        = "boolean"
    required    = true
    description = "Whether customers are impacted by this incident"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.datadoghq.com/api/v2/incidents"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }

    body {
      kind = "json"
      value = {
        data = {
          type = "incidents"
          attributes = {
            title             = "{{ args.title }}"
            customer_impacted = "{{ args.customer_impacted }}"
          }
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Created incident [{{ result.data.id }}]: {{ result.data.attributes.title }}"
  }
}

command "trigger_synthetics" {
  title       = "Trigger synthetic test"
  summary     = "Trigger a Datadog synthetic test on demand"
  description = "Manually trigger a synthetic monitoring test by its public ID."
  categories  = ["synthetics"]

  annotations {
    mode    = "write"
    secrets = ["datadog.api_key", "datadog.app_key"]
  }

  param "public_id" {
    type        = "string"
    required    = true
    description = "Public ID of the synthetic test to trigger"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.datadoghq.com/api/v1/synthetics/tests/trigger"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "DD-API-KEY"
      secret   = "datadog.api_key"
    }

    headers = {
      "DD-APPLICATION-KEY" = "{{ secrets.datadog.app_key }}"
    }

    body {
      kind = "json"
      value = {
        tests = [{
          public_id = "{{ args.public_id }}"
        }]
      }
    }
  }

  result {
    decode = "json"
    output = "Triggered batch {{ result.batch_id }} — {{ result.results | length }} test(s) queued."
  }
}
