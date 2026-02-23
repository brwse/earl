version = 1
provider = "resend"
categories = ["email", "communications"]

command "send_email" {
  title       = "Send email"
  summary     = "Send an email via Resend"
  description = "Send a transactional email with the specified sender, recipient, subject, and HTML body."
  categories  = ["email"]

  annotations {
    mode    = "write"
    secrets = ["resend.api_key"]
  }

  param "from" {
    type        = "string"
    required    = true
    description = "Sender email address (e.g. 'Name <email@domain.com>')"
  }

  param "to" {
    type        = "string"
    required    = true
    description = "Recipient email address"
  }

  param "subject" {
    type        = "string"
    required    = true
    description = "Email subject line"
  }

  param "html" {
    type        = "string"
    required    = true
    description = "HTML body content"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.resend.com/emails"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }

    body {
      kind = "json"
      value = {
        from    = "{{ args.from }}"
        to      = "{{ args.to }}"
        subject = "{{ args.subject }}"
        html    = "{{ args.html }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Email sent: {{ result.id }}"
  }
}

command "get_email" {
  title       = "Get email"
  summary     = "Retrieve details of a sent email"
  description = "Fetch the full details and delivery status of a previously sent email by its ID."
  categories  = ["email"]

  annotations {
    mode    = "read"
    secrets = ["resend.api_key"]
  }

  param "email_id" {
    type        = "string"
    required    = true
    description = "The email ID to retrieve"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.resend.com/emails/{{ args.email_id }}"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }
  }

  result {
    decode = "json"
    output = "Email {{ result.id }} — From: {{ result.from }}, Subject: {{ result.subject }}, Status: {{ result.last_event }}"
  }
}

command "list_emails" {
  title       = "List emails"
  summary     = "List sent emails"
  description = "Retrieve a paginated list of emails that have been sent."
  categories  = ["email"]

  annotations {
    mode    = "read"
    secrets = ["resend.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of results to return (1-100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.resend.com/emails"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }

    query = {
      limit = "{{ args.limit }}"
    }
  }

  result {
    decode = "json"
    output = "{{ result.data | length }} emails returned (has_more: {{ result.has_more }})"
  }
}

command "cancel_email" {
  title       = "Cancel email"
  summary     = "Cancel a scheduled email"
  description = "Cancel a previously scheduled email before it is sent."
  categories  = ["email"]

  annotations {
    mode    = "write"
    secrets = ["resend.api_key"]
  }

  param "email_id" {
    type        = "string"
    required    = true
    description = "The scheduled email ID to cancel"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.resend.com/emails/{{ args.email_id }}/cancel"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }
  }

  result {
    decode = "json"
    output = "Cancelled email: {{ result.id }}"
  }
}

command "list_domains" {
  title       = "List domains"
  summary     = "List all sending domains"
  description = "Retrieve a paginated list of domains configured in your Resend account."
  categories  = ["domains"]

  annotations {
    mode    = "read"
    secrets = ["resend.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of results to return (1-100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.resend.com/domains"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }

    query = {
      limit = "{{ args.limit }}"
    }
  }

  result {
    decode = "json"
    output = "{{ result.data | length }} domains returned (has_more: {{ result.has_more }})"
  }
}

command "get_domain" {
  title       = "Get domain"
  summary     = "Get details of a sending domain"
  description = "Retrieve the full details, DNS records, and verification status of a domain."
  categories  = ["domains"]

  annotations {
    mode    = "read"
    secrets = ["resend.api_key"]
  }

  param "domain_id" {
    type        = "string"
    required    = true
    description = "The domain ID to retrieve"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.resend.com/domains/{{ args.domain_id }}"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }
  }

  result {
    decode = "json"
    output = "Domain: {{ result.name }} ({{ result.id }}) — Status: {{ result.status }}, Region: {{ result.region }}"
  }
}

command "create_domain" {
  title       = "Create domain"
  summary     = "Add a new sending domain"
  description = "Register a new domain for sending emails through Resend."
  categories  = ["domains"]

  annotations {
    mode    = "write"
    secrets = ["resend.api_key"]
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Domain name (e.g. 'example.com')"
  }

  param "region" {
    type        = "string"
    required    = false
    default     = "us-east-1"
    description = "Region: us-east-1, eu-west-1, sa-east-1, or ap-northeast-1"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.resend.com/domains"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }

    body {
      kind = "json"
      value = {
        name   = "{{ args.name }}"
        region = "{{ args.region }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created domain: {{ result.name }} ({{ result.id }}) — Status: {{ result.status }}"
  }
}

command "verify_domain" {
  title       = "Verify domain"
  summary     = "Trigger DNS verification for a domain"
  description = "Initiate the DNS verification process for a domain to enable email sending."
  categories  = ["domains"]

  annotations {
    mode    = "write"
    secrets = ["resend.api_key"]
  }

  param "domain_id" {
    type        = "string"
    required    = true
    description = "The domain ID to verify"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.resend.com/domains/{{ args.domain_id }}/verify"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }
  }

  result {
    decode = "json"
    output = "Verification initiated for domain: {{ result.id }}"
  }
}

command "delete_domain" {
  title       = "Delete domain"
  summary     = "Remove a sending domain"
  description = "Permanently delete a domain from your Resend account."
  categories  = ["domains"]

  annotations {
    mode    = "write"
    secrets = ["resend.api_key"]
  }

  param "domain_id" {
    type        = "string"
    required    = true
    description = "The domain ID to delete"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.resend.com/domains/{{ args.domain_id }}"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }
  }

  result {
    decode = "json"
    output = "Deleted domain: {{ result.id }}"
  }
}

command "list_contacts" {
  title       = "List contacts"
  summary     = "List all contacts"
  description = "Retrieve a paginated list of contacts in your Resend account."
  categories  = ["contacts"]

  annotations {
    mode    = "read"
    secrets = ["resend.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Number of results to return (1-100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.resend.com/contacts"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }

    query = {
      limit = "{{ args.limit }}"
    }
  }

  result {
    decode = "json"
    output = "{{ result.data | length }} contacts returned (has_more: {{ result.has_more }})"
  }
}

command "get_contact" {
  title       = "Get contact"
  summary     = "Retrieve a contact by ID"
  description = "Fetch the full details of a contact by their ID or email address."
  categories  = ["contacts"]

  annotations {
    mode    = "read"
    secrets = ["resend.api_key"]
  }

  param "contact_id" {
    type        = "string"
    required    = true
    description = "Contact ID or email address"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.resend.com/contacts/{{ args.contact_id }}"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }
  }

  result {
    decode = "json"
    output = "Contact: {{ result.email }} ({{ result.id }}) — {{ result.first_name }} {{ result.last_name }}, Unsubscribed: {{ result.unsubscribed }}"
  }
}

command "create_contact" {
  title       = "Create contact"
  summary     = "Add a new contact"
  description = "Create a new contact with an email address and optional name."
  categories  = ["contacts"]

  annotations {
    mode    = "write"
    secrets = ["resend.api_key"]
  }

  param "email" {
    type        = "string"
    required    = true
    description = "Contact email address"
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
    method   = "POST"
    url      = "https://api.resend.com/contacts"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }

    body {
      kind = "json"
      value = {
        email      = "{{ args.email }}"
        first_name = "{{ args.first_name }}"
        last_name  = "{{ args.last_name }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created contact: {{ result.id }}"
  }
}

command "update_contact" {
  title       = "Update contact"
  summary     = "Update an existing contact"
  description = "Update the name or subscription status of a contact."
  categories  = ["contacts"]

  annotations {
    mode    = "write"
    secrets = ["resend.api_key"]
  }

  param "contact_id" {
    type        = "string"
    required    = true
    description = "Contact ID or email address"
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

  param "unsubscribed" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Set to true to unsubscribe from all broadcasts"
  }

  operation {
    protocol = "http"
    method   = "PATCH"
    url      = "https://api.resend.com/contacts/{{ args.contact_id }}"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }

    body {
      kind = "json"
      value = {
        first_name   = "{{ args.first_name }}"
        last_name    = "{{ args.last_name }}"
        unsubscribed = "{{ args.unsubscribed }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Updated contact: {{ result.id }}"
  }
}

command "delete_contact" {
  title       = "Delete contact"
  summary     = "Remove a contact"
  description = "Permanently delete a contact by their ID or email address."
  categories  = ["contacts"]

  annotations {
    mode    = "write"
    secrets = ["resend.api_key"]
  }

  param "contact_id" {
    type        = "string"
    required    = true
    description = "Contact ID or email address to delete"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.resend.com/contacts/{{ args.contact_id }}"

    auth {
      kind   = "bearer"
      secret = "resend.api_key"
    }
  }

  result {
    decode = "json"
    output = "Deleted contact: {{ result.contact }}"
  }
}
