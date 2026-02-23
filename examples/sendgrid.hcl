version = 1
provider = "sendgrid"
categories = ["email", "marketing"]

command "send_mail" {
  title       = "Send email"
  summary     = "Send an email via SendGrid"
  description = "Send a transactional email through the SendGrid Mail Send API. Returns 202 on success."
  categories  = ["email"]

  annotations {
    mode    = "write"
    secrets = ["sendgrid.api_key"]
  }

  param "to" {
    type        = "string"
    required    = true
    description = "Recipient email address"
  }

  param "from" {
    type        = "string"
    required    = true
    description = "Verified sender email address"
  }

  param "subject" {
    type        = "string"
    required    = true
    description = "Email subject line"
  }

  param "body_text" {
    type        = "string"
    required    = true
    description = "Plain text email body"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.sendgrid.com/v3/mail/send"

    auth {
      kind   = "bearer"
      secret = "sendgrid.api_key"
    }

    body {
      kind = "json"
      value = {
        personalizations = [
          {
            to = [
              {
                email = "{{ args.to }}"
              }
            ]
          }
        ]
        from = {
          email = "{{ args.from }}"
        }
        subject = "{{ args.subject }}"
        content = [
          {
            type  = "text/plain"
            value = "{{ args.body_text }}"
          }
        ]
      }
    }
  }

  result {
    decode = "json"
    output = "Email sent to {{ args.to }} from {{ args.from }} (subject: \"{{ args.subject }}\")"
  }
}

command "search_contacts" {
  title       = "Search contacts"
  summary     = "Search contacts using SGQL query"
  description = "Search marketing contacts using SendGrid Query Language (SGQL). Example: email LIKE '%example.com' or first_name = 'John'."
  categories  = ["marketing"]

  annotations {
    mode    = "read"
    secrets = ["sendgrid.api_key"]
  }

  param "query" {
    type        = "string"
    required    = true
    description = "SGQL query (e.g. \"email LIKE '%example.com'\")"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.sendgrid.com/v3/marketing/contacts/search"

    auth {
      kind   = "bearer"
      secret = "sendgrid.api_key"
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
    output = "Found {{ result.contact_count }} contacts:\n{% for c in result.result %}- {{ c.email }} ({{ c.first_name }} {{ c.last_name }}) [ID: {{ c.id }}]\n{% endfor %}"
  }
}

command "get_contact" {
  title       = "Get contact"
  summary     = "Get a contact by ID"
  description = "Retrieve a single marketing contact by its UUID."
  categories  = ["marketing"]

  annotations {
    mode    = "read"
    secrets = ["sendgrid.api_key"]
  }

  param "id" {
    type        = "string"
    required    = true
    description = "Contact UUID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.sendgrid.com/v3/marketing/contacts/{{ args.id }}"

    auth {
      kind   = "bearer"
      secret = "sendgrid.api_key"
    }
  }

  result {
    decode = "json"
    output = "Contact: {{ result.email }}\nName: {{ result.first_name }} {{ result.last_name }}\nCreated: {{ result.created_at }}\nUpdated: {{ result.updated_at }}"
  }
}

command "upsert_contacts" {
  title       = "Upsert contacts"
  summary     = "Create or update a contact"
  description = "Add or update a marketing contact. The email address is the upsert key."
  categories  = ["marketing"]

  annotations {
    mode    = "write"
    secrets = ["sendgrid.api_key"]
  }

  param "email" {
    type        = "string"
    required    = true
    description = "Contact email address (upsert key)"
  }

  param "first_name" {
    type        = "string"
    required    = false
    default     = ""
    description = "Contact first name"
  }

  param "last_name" {
    type        = "string"
    required    = false
    default     = ""
    description = "Contact last name"
  }

  operation {
    protocol = "http"
    method   = "PUT"
    url      = "https://api.sendgrid.com/v3/marketing/contacts"

    auth {
      kind   = "bearer"
      secret = "sendgrid.api_key"
    }

    body {
      kind = "json"
      value = {
        contacts = [
          {
            email      = "{{ args.email }}"
            first_name = "{{ args.first_name }}"
            last_name  = "{{ args.last_name }}"
          }
        ]
      }
    }
  }

  result {
    decode = "json"
    output = "Contact upsert queued. Job ID: {{ result.job_id }}"
  }
}

command "delete_contacts" {
  title       = "Delete contacts"
  summary     = "Delete contacts by IDs"
  description = "Delete one or more marketing contacts by their UUIDs."
  categories  = ["marketing"]

  annotations {
    mode    = "write"
    secrets = ["sendgrid.api_key"]
  }

  param "ids" {
    type        = "string"
    required    = true
    description = "Comma-separated contact UUIDs to delete"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.sendgrid.com/v3/marketing/contacts"

    auth {
      kind   = "bearer"
      secret = "sendgrid.api_key"
    }

    query = {
      ids = "{{ args.ids }}"
    }
  }

  result {
    decode = "json"
    output = "Contact deletion queued. Job ID: {{ result.job_id }}"
  }
}

command "list_contact_lists" {
  title       = "List contact lists"
  summary     = "List all marketing contact lists"
  description = "Retrieve all marketing contact lists with their contact counts."
  categories  = ["marketing"]

  annotations {
    mode    = "read"
    secrets = ["sendgrid.api_key"]
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Results per page (1-1000)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.sendgrid.com/v3/marketing/lists"

    auth {
      kind   = "bearer"
      secret = "sendgrid.api_key"
    }

    query = {
      page_size = "{{ args.page_size }}"
    }
  }

  result {
    decode = "json"
    output = "{{ result.result | length }} contact lists:\n{% for l in result.result %}- {{ l.name }} ({{ l.contact_count }} contacts) [ID: {{ l.id }}]\n{% endfor %}"
  }
}

command "list_templates" {
  title       = "List templates"
  summary     = "List email templates"
  description = "Retrieve transactional and dynamic email templates."
  categories  = ["email"]

  annotations {
    mode    = "read"
    secrets = ["sendgrid.api_key"]
  }

  param "generations" {
    type        = "string"
    required    = false
    default     = "legacy,dynamic"
    description = "Filter by generation: legacy, dynamic, or both"
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Results per page (1-200)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.sendgrid.com/v3/templates"

    auth {
      kind   = "bearer"
      secret = "sendgrid.api_key"
    }

    query = {
      generations = "{{ args.generations }}"
      page_size   = "{{ args.page_size }}"
    }
  }

  result {
    decode = "json"
    output = "{{ result.result | length }} templates:\n{% for t in result.result %}- {{ t.name }} ({{ t.generation }}) [ID: {{ t.id }}]\n{% endfor %}"
  }
}

command "get_global_stats" {
  title       = "Get global stats"
  summary     = "Get email sending statistics"
  description = "Retrieve global email statistics for a date range including requests, deliveries, opens, clicks, bounces, and spam reports."
  categories  = ["email"]

  annotations {
    mode    = "read"
    secrets = ["sendgrid.api_key"]
  }

  param "start_date" {
    type        = "string"
    required    = true
    description = "Start date in YYYY-MM-DD format"
  }

  param "end_date" {
    type        = "string"
    required    = false
    description = "End date in YYYY-MM-DD format (defaults to today)"
  }

  param "aggregated_by" {
    type        = "string"
    required    = false
    description = "Aggregation period: day, week, or month"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.sendgrid.com/v3/stats"

    auth {
      kind   = "bearer"
      secret = "sendgrid.api_key"
    }

    query = {
      start_date = "{{ args.start_date }}"
    }
  }

  result {
    decode = "json"
    output = "Email stats from {{ args.start_date }}:\n{% for day in result %}{{ day.date }}: {{ day.stats[0].metrics.requests }} sent, {{ day.stats[0].metrics.delivered }} delivered, {{ day.stats[0].metrics.opens }} opens, {{ day.stats[0].metrics.clicks }} clicks, {{ day.stats[0].metrics.bounces }} bounces\n{% endfor %}"
  }
}

command "list_email_activity" {
  title       = "List email activity"
  summary     = "Search recent email activity"
  description = "Query recent email activity using SGQL. Filterable by from_email, to_email, subject, status (delivered/not_delivered/processing), and last_event_time. Requires the Email Activity History add-on."
  categories  = ["email"]

  annotations {
    mode    = "read"
    secrets = ["sendgrid.api_key"]
  }

  param "query" {
    type        = "string"
    required    = true
    description = "SGQL query for email activity (e.g. \"status = 'delivered'\")"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of messages to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.sendgrid.com/v3/messages"

    auth {
      kind   = "bearer"
      secret = "sendgrid.api_key"
    }

    query = {
      query = "{{ args.query }}"
      limit = "{{ args.limit }}"
    }
  }

  result {
    decode = "json"
    output = "{% for m in result.messages %}- {{ m.status | upper }}: {{ m.from_email }} -> {{ m.to_email }} \"{{ m.subject }}\" ({{ m.last_event_time }})\n{% endfor %}"
  }
}

command "list_bounces" {
  title       = "List bounces"
  summary     = "List bounced email addresses"
  description = "Retrieve email addresses that have bounced, with the bounce reason and status."
  categories  = ["email"]

  annotations {
    mode    = "read"
    secrets = ["sendgrid.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Maximum number of results (1-500)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.sendgrid.com/v3/suppression/bounces"

    auth {
      kind   = "bearer"
      secret = "sendgrid.api_key"
    }

    query = {
      limit = "{{ args.limit }}"
    }
  }

  result {
    decode = "json"
    output = "{{ result | length }} bounces:\n{% for b in result %}- {{ b.email }} -- {{ b.reason }} ({{ b.created }})\n{% endfor %}"
  }
}

command "list_global_suppressions" {
  title       = "List global suppressions"
  summary     = "List globally unsubscribed emails"
  description = "Retrieve email addresses that have globally unsubscribed from all emails."
  categories  = ["email"]

  annotations {
    mode    = "read"
    secrets = ["sendgrid.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Maximum number of results (1-500)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.sendgrid.com/v3/suppression/unsubscribes"

    auth {
      kind   = "bearer"
      secret = "sendgrid.api_key"
    }

    query = {
      limit = "{{ args.limit }}"
    }
  }

  result {
    decode = "json"
    output = "{{ result | length }} global suppressions:\n{% for s in result %}- {{ s.email }} (suppressed {{ s.created }})\n{% endfor %}"
  }
}

command "list_verified_senders" {
  title       = "List verified senders"
  summary     = "List verified sender identities"
  description = "Retrieve all verified sender identities and their verification status."
  categories  = ["email"]

  annotations {
    mode    = "read"
    secrets = ["sendgrid.api_key"]
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.sendgrid.com/v3/verified_senders"

    auth {
      kind   = "bearer"
      secret = "sendgrid.api_key"
    }
  }

  result {
    decode = "json"
    output = "{% for s in result.results %}- {{ s.from_name }} <{{ s.from_email }}> [ID: {{ s.id }}]\n{% endfor %}"
  }
}

command "list_api_keys" {
  title       = "List API keys"
  summary     = "List all API keys"
  description = "Retrieve all API keys associated with the account."
  categories  = ["email"]

  annotations {
    mode    = "read"
    secrets = ["sendgrid.api_key"]
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.sendgrid.com/v3/api_keys"

    auth {
      kind   = "bearer"
      secret = "sendgrid.api_key"
    }
  }

  result {
    decode = "json"
    output = "{{ result.result | length }} API keys:\n{% for k in result.result %}- {{ k.name }} [ID: {{ k.api_key_id }}]\n{% endfor %}"
  }
}
