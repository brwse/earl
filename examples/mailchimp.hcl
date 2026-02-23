version = 1
provider = "mailchimp"
categories = ["email", "marketing"]

command "ping" {
  title       = "Ping"
  summary     = "Check Mailchimp API connectivity"
  description = "Verify that your API key is valid and the Mailchimp API is reachable."
  categories  = ["utility"]

  annotations {
    mode    = "read"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/ping"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }
  }

  result {
    decode = "json"
    output = "Status: {{ result.health_status }}"
  }
}

command "list_audiences" {
  title       = "List audiences"
  summary     = "List all audiences (lists) in your account"
  description = "Retrieve audiences with member counts and stats. Supports pagination."
  categories  = ["audiences"]

  annotations {
    mode    = "read"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "count" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of results to return"
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
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/lists"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }

    query = {
      count  = "{{ args.count }}"
      offset = "{{ args.offset }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.total_items }} audiences:\n{% for list in result.lists %}  - {{ list.name }} ({{ list.id }}) — {{ list.stats.member_count }} members\n{% endfor %}"
  }
}

command "get_audience" {
  title       = "Get audience"
  summary     = "Get details for a specific audience"
  description = "Retrieve detailed information about an audience including stats and settings."
  categories  = ["audiences"]

  annotations {
    mode    = "read"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "list_id" {
    type        = "string"
    required    = true
    description = "Audience/list ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/lists/{{ args.list_id }}"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }
  }

  result {
    decode = "json"
    output = "Audience: {{ result.name }} ({{ result.id }})\nMembers: {{ result.stats.member_count }}\nOpen rate: {{ result.stats.open_rate }}\nClick rate: {{ result.stats.click_rate }}\nCreated: {{ result.date_created }}"
  }
}

command "list_members" {
  title       = "List members"
  summary     = "List members of an audience"
  description = "Retrieve subscribers for a specific audience. Supports filtering by status and pagination."
  categories  = ["members"]

  annotations {
    mode    = "read"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "list_id" {
    type        = "string"
    required    = true
    description = "Audience/list ID"
  }

  param "count" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of results to return (max 50)"
  }

  param "offset" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Pagination offset"
  }

  param "status" {
    type        = "string"
    required    = false
    description = "Filter by status: subscribed, unsubscribed, cleaned, pending, transactional, archived"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/lists/{{ args.list_id }}/members"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }

    query = {
      count  = "{{ args.count }}"
      offset = "{{ args.offset }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.total_items }} members:\n{% for m in result.members %}  - {{ m.email_address }} ({{ m.status }}) — {{ m.merge_fields.FNAME }} {{ m.merge_fields.LNAME }}\n{% endfor %}"
  }
}

command "search_members" {
  title       = "Search members"
  summary     = "Search for members by email or name"
  description = "Search across all audiences for members matching a query string."
  categories  = ["members"]

  annotations {
    mode    = "read"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "query" {
    type        = "string"
    required    = true
    description = "Email address or name to search for"
  }

  param "list_id" {
    type        = "string"
    required    = false
    description = "Restrict search to a specific audience"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/search-members"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }

    query = {
      query = "{{ args.query }}"
    }
  }

  result {
    decode = "json"
    output = "Exact matches: {{ result.exact_matches.total_items }}\n{% for m in result.exact_matches.members %}  - {{ m.email_address }} ({{ m.list_id }})\n{% endfor %}Full search: {{ result.full_search.total_items }} results"
  }
}

command "add_member" {
  title       = "Add member"
  summary     = "Add a subscriber to an audience"
  description = "Add a new member to a Mailchimp audience with the specified subscription status."
  categories  = ["members"]

  annotations {
    mode    = "write"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "list_id" {
    type        = "string"
    required    = true
    description = "Audience/list ID"
  }

  param "email_address" {
    type        = "string"
    required    = true
    description = "Subscriber email address"
  }

  param "status" {
    type        = "string"
    required    = true
    description = "Subscription status: subscribed, pending, unsubscribed, or transactional"
  }

  param "first_name" {
    type        = "string"
    required    = false
    default     = ""
    description = "Subscriber first name"
  }

  param "last_name" {
    type        = "string"
    required    = false
    default     = ""
    description = "Subscriber last name"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/lists/{{ args.list_id }}/members"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }

    body {
      kind = "json"
      value = {
        email_address = "{{ args.email_address }}"
        status        = "{{ args.status }}"
        merge_fields = {
          FNAME = "{{ args.first_name }}"
          LNAME = "{{ args.last_name }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Added {{ result.email_address }} to list {{ result.list_id }}\nStatus: {{ result.status }}\nID: {{ result.id }}"
  }
}

command "update_member" {
  title       = "Update member"
  summary     = "Update a subscriber in an audience"
  description = "Update an existing member's information. The subscriber_hash is the MD5 hash of the lowercased email address."
  categories  = ["members"]

  annotations {
    mode    = "write"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "list_id" {
    type        = "string"
    required    = true
    description = "Audience/list ID"
  }

  param "subscriber_hash" {
    type        = "string"
    required    = true
    description = "MD5 hash of the lowercased subscriber email address"
  }

  param "status" {
    type        = "string"
    required    = false
    description = "New subscription status: subscribed, unsubscribed, cleaned, pending"
  }

  param "first_name" {
    type        = "string"
    required    = false
    default     = ""
    description = "Updated first name"
  }

  param "last_name" {
    type        = "string"
    required    = false
    default     = ""
    description = "Updated last name"
  }

  operation {
    protocol = "http"
    method   = "PATCH"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/lists/{{ args.list_id }}/members/{{ args.subscriber_hash }}"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }

    body {
      kind = "json"
      value = {
        merge_fields = {
          FNAME = "{{ args.first_name }}"
          LNAME = "{{ args.last_name }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Updated {{ result.email_address }}\nStatus: {{ result.status }}"
  }
}

command "archive_member" {
  title       = "Archive member"
  summary     = "Archive a subscriber from an audience"
  description = "Archive (soft-delete) a member from an audience. The subscriber_hash is the MD5 hash of the lowercased email address."
  categories  = ["members"]

  annotations {
    mode    = "write"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "list_id" {
    type        = "string"
    required    = true
    description = "Audience/list ID"
  }

  param "subscriber_hash" {
    type        = "string"
    required    = true
    description = "MD5 hash of the lowercased subscriber email address"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/lists/{{ args.list_id }}/members/{{ args.subscriber_hash }}"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }
  }

  result {
    decode = "json"
    output = "Member archived from list {{ args.list_id }}"
  }
}

command "list_campaigns" {
  title       = "List campaigns"
  summary     = "List email campaigns"
  description = "Retrieve campaigns with optional filtering by status and type. Supports pagination."
  categories  = ["campaigns"]

  annotations {
    mode    = "read"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "count" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of results to return"
  }

  param "offset" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Pagination offset"
  }

  param "status" {
    type        = "string"
    required    = false
    description = "Filter by status: save, sent, sending, schedule, paused"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/campaigns"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }

    query = {
      count  = "{{ args.count }}"
      offset = "{{ args.offset }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.total_items }} campaigns:\n{% for c in result.campaigns %}  - {{ c.settings.title }} [{{ c.status }}] ({{ c.id }}) — {{ c.emails_sent }} sent\n{% endfor %}"
  }
}

command "get_campaign" {
  title       = "Get campaign"
  summary     = "Get details for a specific campaign"
  description = "Retrieve detailed information about a campaign including settings, recipients, and send stats."
  categories  = ["campaigns"]

  annotations {
    mode    = "read"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "campaign_id" {
    type        = "string"
    required    = true
    description = "Campaign ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/campaigns/{{ args.campaign_id }}"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }
  }

  result {
    decode = "json"
    output = "Campaign: {{ result.settings.title }}\nSubject: {{ result.settings.subject_line }}\nStatus: {{ result.status }}\nType: {{ result.type }}\nList: {{ result.recipients.list_name }} ({{ result.recipients.list_id }})\nEmails sent: {{ result.emails_sent }}\nSend time: {{ result.send_time }}"
  }
}

command "create_campaign" {
  title       = "Create campaign"
  summary     = "Create a new email campaign"
  description = "Create a new campaign targeting a specific audience. After creating, set content and then send."
  categories  = ["campaigns"]

  annotations {
    mode    = "write"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "type" {
    type        = "string"
    required    = true
    description = "Campaign type: regular, plaintext, rss, variate, or absplit"
  }

  param "list_id" {
    type        = "string"
    required    = true
    description = "Audience/list ID to send to"
  }

  param "subject_line" {
    type        = "string"
    required    = false
    default     = ""
    description = "Email subject line"
  }

  param "title" {
    type        = "string"
    required    = false
    default     = ""
    description = "Internal campaign title"
  }

  param "from_name" {
    type        = "string"
    required    = false
    default     = ""
    description = "From name for the campaign"
  }

  param "reply_to" {
    type        = "string"
    required    = false
    default     = ""
    description = "Reply-to email address"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/campaigns"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }

    body {
      kind = "json"
      value = {
        type = "{{ args.type }}"
        recipients = {
          list_id = "{{ args.list_id }}"
        }
        settings = {
          subject_line = "{{ args.subject_line }}"
          title        = "{{ args.title }}"
          from_name    = "{{ args.from_name }}"
          reply_to     = "{{ args.reply_to }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Created campaign {{ result.id }}\nTitle: {{ result.settings.title }}\nStatus: {{ result.status }}"
  }
}

command "send_campaign" {
  title       = "Send campaign"
  summary     = "Send an email campaign"
  description = "Send a campaign immediately. The campaign must have content set before sending."
  categories  = ["campaigns"]

  annotations {
    mode    = "write"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "campaign_id" {
    type        = "string"
    required    = true
    description = "Campaign ID to send"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/campaigns/{{ args.campaign_id }}/actions/send"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }
  }

  result {
    decode = "json"
    output = "Campaign {{ args.campaign_id }} has been sent."
  }
}

command "send_test_email" {
  title       = "Send test email"
  summary     = "Send a test email for a campaign"
  description = "Send a test version of a campaign to specified email addresses before sending to the full audience."
  categories  = ["campaigns"]

  annotations {
    mode    = "write"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "campaign_id" {
    type        = "string"
    required    = true
    description = "Campaign ID to test"
  }

  param "test_emails" {
    type        = "array"
    required    = true
    description = "List of email addresses to send the test to"
  }

  param "send_type" {
    type        = "string"
    required    = true
    description = "Type of test email to send: html or plaintext"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/campaigns/{{ args.campaign_id }}/actions/test"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }

    body {
      kind = "json"
      value = {
        test_emails = "{{ args.test_emails }}"
        send_type   = "{{ args.send_type }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Test email sent for campaign {{ args.campaign_id }}"
  }
}

command "list_templates" {
  title       = "List templates"
  summary     = "List email templates"
  description = "Retrieve email templates in your account. Supports filtering by type and pagination."
  categories  = ["templates"]

  annotations {
    mode    = "read"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "count" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of results to return"
  }

  param "offset" {
    type        = "integer"
    required    = false
    default     = 0
    description = "Pagination offset"
  }

  param "type" {
    type        = "string"
    required    = false
    description = "Filter by type: user, base, or gallery"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/templates"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }

    query = {
      count  = "{{ args.count }}"
      offset = "{{ args.offset }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.total_items }} templates:\n{% for t in result.templates %}  - {{ t.name }} ({{ t.id }}) [{{ t.type }}] — edited {{ t.date_edited }}\n{% endfor %}"
  }
}

command "delete_campaign" {
  title       = "Delete campaign"
  summary     = "Delete a campaign"
  description = "Permanently delete a campaign. This cannot be undone."
  categories  = ["campaigns"]

  annotations {
    mode    = "write"
    secrets = ["mailchimp.api_key"]
  }

  param "dc" {
    type        = "string"
    required    = true
    description = "Datacenter slug from your API key (e.g. 'us6')"
  }

  param "campaign_id" {
    type        = "string"
    required    = true
    description = "Campaign ID to delete"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://{{ args.dc }}.api.mailchimp.com/3.0/campaigns/{{ args.campaign_id }}"

    auth {
      kind            = "basic"
      username        = "anystring"
      password_secret = "mailchimp.api_key"
    }
  }

  result {
    decode = "json"
    output = "Campaign {{ args.campaign_id }} deleted."
  }
}
