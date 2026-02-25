version = 1
provider = "hubspot"
categories = ["crm", "sales", "marketing"]

command "list_contacts" {
  title       = "List contacts"
  summary     = "List CRM contacts with optional property selection"
  description = "Retrieve a paginated list of contacts from HubSpot CRM. Use the properties parameter to specify which contact fields to return."
  categories  = ["crm"]

  annotations {
    mode    = "read"
    secrets = ["hubspot.token"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Records per page (max 100)"
  }

  param "after" {
    type        = "string"
    required    = false
    default     = ""
    description = "Pagination cursor from a previous response"
  }

  param "properties" {
    type        = "string"
    required    = false
    default     = "firstname,lastname,email"
    description = "Comma-separated property names to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.hubapi.com/crm/v3/objects/contacts"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }

    query = {
      limit      = "{{ args.limit }}"
      after      = "{{ args.after }}"
      properties = "{{ args.properties }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.results | length }} contacts.{% for c in result.results %}\n- [{{ c.id }}] {{ c.properties.firstname }} {{ c.properties.lastname }} <{{ c.properties.email }}>{% endfor %}"
  }
}

command "get_contact" {
  title       = "Get contact"
  summary     = "Retrieve a single contact by ID"
  description = "Fetch a HubSpot contact by its ID or by a custom identity property such as email."
  categories  = ["crm"]

  annotations {
    mode    = "read"
    secrets = ["hubspot.token"]
  }

  param "contact_id" {
    type        = "string"
    required    = true
    description = "HubSpot contact ID or value of id_property"
  }

  param "id_property" {
    type        = "string"
    required    = false
    default     = ""
    description = "Custom property to use as lookup key (e.g. email)"
  }

  param "properties" {
    type        = "string"
    required    = false
    default     = "firstname,lastname,email,phone,company,lifecyclestage"
    description = "Comma-separated property names to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.hubapi.com/crm/v3/objects/contacts/{{ args.contact_id }}"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }

    query = {
      idProperty = "{{ args.id_property }}"
      properties = "{{ args.properties }}"
    }
  }

  result {
    decode = "json"
    output = "Contact {{ result.id }}:\n  Name: {{ result.properties.firstname }} {{ result.properties.lastname }}\n  Email: {{ result.properties.email }}\n  Phone: {{ result.properties.phone }}\n  Company: {{ result.properties.company }}\n  Lifecycle: {{ result.properties.lifecyclestage }}\n  Created: {{ result.createdAt }}"
  }
}

command "create_contact" {
  title       = "Create contact"
  summary     = "Create a new CRM contact"
  description = "Create a new contact in HubSpot CRM with the specified properties."
  categories  = ["crm"]

  annotations {
    mode    = "write"
    secrets = ["hubspot.token"]
  }

  param "email" {
    type        = "string"
    required    = true
    description = "Contact email address"
  }

  param "firstname" {
    type        = "string"
    required    = false
    default     = ""
    description = "First name"
  }

  param "lastname" {
    type        = "string"
    required    = false
    default     = ""
    description = "Last name"
  }

  param "phone" {
    type        = "string"
    required    = false
    default     = ""
    description = "Phone number"
  }

  param "company" {
    type        = "string"
    required    = false
    default     = ""
    description = "Company name"
  }

  param "jobtitle" {
    type        = "string"
    required    = false
    default     = ""
    description = "Job title"
  }

  param "lifecyclestage" {
    type        = "string"
    required    = false
    default     = ""
    description = "Lifecycle stage (e.g. lead, customer, subscriber)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.hubapi.com/crm/v3/objects/contacts"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }

    body {
      kind = "json"
      value = {
        properties = {
          email          = "{{ args.email }}"
          firstname      = "{{ args.firstname }}"
          lastname       = "{{ args.lastname }}"
          phone          = "{{ args.phone }}"
          company        = "{{ args.company }}"
          jobtitle       = "{{ args.jobtitle }}"
          lifecyclestage = "{{ args.lifecyclestage }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Created contact {{ result.id }}: {{ result.properties.firstname }} {{ result.properties.lastname }} <{{ result.properties.email }}>"
  }
}

command "update_contact" {
  title       = "Update contact"
  summary     = "Update properties on an existing contact"
  description = "Update one or more properties on a HubSpot contact. Pass a JSON object of property names to values."
  categories  = ["crm"]

  annotations {
    mode    = "write"
    secrets = ["hubspot.token"]
  }

  param "contact_id" {
    type        = "string"
    required    = true
    description = "HubSpot contact ID"
  }

  param "properties" {
    type        = "object"
    required    = true
    description = "JSON object of property names to values (e.g. {\"email\": \"new@example.com\"})"
  }

  operation {
    protocol = "http"
    method   = "PATCH"
    url      = "https://api.hubapi.com/crm/v3/objects/contacts/{{ args.contact_id }}"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }

    body {
      kind = "json"
      value = {
        properties = "{{ args.properties | tojson }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Updated contact {{ result.id }}: {{ result.properties.firstname }} {{ result.properties.lastname }}"
  }
}

command "delete_contact" {
  title       = "Delete contact"
  summary     = "Archive a contact by ID"
  description = "Archive (soft-delete) a contact in HubSpot CRM. The contact can be restored from the recycling bin."
  categories  = ["crm"]

  annotations {
    mode    = "write"
    secrets = ["hubspot.token"]
  }

  param "contact_id" {
    type        = "string"
    required    = true
    description = "HubSpot contact ID to archive"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.hubapi.com/crm/v3/objects/contacts/{{ args.contact_id }}"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }
  }

  result {
    decode = "json"
    output = "Archived contact {{ args.contact_id }}."
  }
}

command "search_contacts" {
  title       = "Search contacts"
  summary     = "Search contacts by query string"
  description = "Search HubSpot contacts using a full-text query. Returns matching contacts with key properties."
  categories  = ["crm", "search"]

  annotations {
    mode    = "read"
    secrets = ["hubspot.token"]
  }

  param "query" {
    type        = "string"
    required    = true
    description = "Full-text search query"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Results per page (max 200)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.hubapi.com/crm/v3/objects/contacts/search"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }

    body {
      kind = "json"
      value = {
        query      = "{{ args.query }}"
        limit      = "{{ args.limit }}"
        properties = ["firstname", "lastname", "email", "company", "lifecyclestage"]
      }
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.total }} contacts.{% for c in result.results %}\n- [{{ c.id }}] {{ c.properties.firstname }} {{ c.properties.lastname }} <{{ c.properties.email }}> ({{ c.properties.lifecyclestage }}){% endfor %}"
  }
}

command "list_companies" {
  title       = "List companies"
  summary     = "List CRM companies"
  description = "Retrieve a paginated list of companies from HubSpot CRM."
  categories  = ["crm"]

  annotations {
    mode    = "read"
    secrets = ["hubspot.token"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Records per page (max 100)"
  }

  param "after" {
    type        = "string"
    required    = false
    default     = ""
    description = "Pagination cursor from a previous response"
  }

  param "properties" {
    type        = "string"
    required    = false
    default     = "name,domain,industry"
    description = "Comma-separated property names to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.hubapi.com/crm/v3/objects/companies"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }

    query = {
      limit      = "{{ args.limit }}"
      after      = "{{ args.after }}"
      properties = "{{ args.properties }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.results | length }} companies.{% for c in result.results %}\n- [{{ c.id }}] {{ c.properties.name }} ({{ c.properties.domain }}) — {{ c.properties.industry }}{% endfor %}"
  }
}

command "create_company" {
  title       = "Create company"
  summary     = "Create a new CRM company"
  description = "Create a new company record in HubSpot CRM."
  categories  = ["crm"]

  annotations {
    mode    = "write"
    secrets = ["hubspot.token"]
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Company name"
  }

  param "domain" {
    type        = "string"
    required    = false
    default     = ""
    description = "Primary website domain"
  }

  param "industry" {
    type        = "string"
    required    = false
    default     = ""
    description = "Industry (e.g. COMPUTER_SOFTWARE)"
  }

  param "phone" {
    type        = "string"
    required    = false
    default     = ""
    description = "Company phone number"
  }

  param "city" {
    type        = "string"
    required    = false
    default     = ""
    description = "City"
  }

  param "state" {
    type        = "string"
    required    = false
    default     = ""
    description = "State or region"
  }

  param "country" {
    type        = "string"
    required    = false
    default     = ""
    description = "Country"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.hubapi.com/crm/v3/objects/companies"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }

    body {
      kind = "json"
      value = {
        properties = {
          name     = "{{ args.name }}"
          domain   = "{{ args.domain }}"
          industry = "{{ args.industry }}"
          phone    = "{{ args.phone }}"
          city     = "{{ args.city }}"
          state    = "{{ args.state }}"
          country  = "{{ args.country }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Created company {{ result.id }}: {{ result.properties.name }} ({{ result.properties.domain }})"
  }
}

command "list_deals" {
  title       = "List deals"
  summary     = "List CRM deals"
  description = "Retrieve a paginated list of deals from HubSpot CRM."
  categories  = ["sales"]

  annotations {
    mode    = "read"
    secrets = ["hubspot.token"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Records per page (max 100)"
  }

  param "after" {
    type        = "string"
    required    = false
    default     = ""
    description = "Pagination cursor from a previous response"
  }

  param "properties" {
    type        = "string"
    required    = false
    default     = "dealname,amount,dealstage"
    description = "Comma-separated property names to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.hubapi.com/crm/v3/objects/deals"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }

    query = {
      limit      = "{{ args.limit }}"
      after      = "{{ args.after }}"
      properties = "{{ args.properties }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.results | length }} deals.{% for d in result.results %}\n- [{{ d.id }}] {{ d.properties.dealname }} — {{ d.properties.amount }} (stage: {{ d.properties.dealstage }}){% endfor %}"
  }
}

command "create_deal" {
  title       = "Create deal"
  summary     = "Create a new sales deal"
  description = "Create a new deal in HubSpot CRM with the specified properties."
  categories  = ["sales"]

  annotations {
    mode    = "write"
    secrets = ["hubspot.token"]
  }

  param "dealname" {
    type        = "string"
    required    = true
    description = "Deal title"
  }

  param "dealstage" {
    type        = "string"
    required    = true
    description = "Pipeline stage internal ID"
  }

  param "pipeline" {
    type        = "string"
    required    = false
    default     = "default"
    description = "Pipeline ID"
  }

  param "amount" {
    type        = "string"
    required    = false
    default     = ""
    description = "Deal value"
  }

  param "closedate" {
    type        = "string"
    required    = false
    default     = ""
    description = "Expected close date (ISO 8601)"
  }

  param "hubspot_owner_id" {
    type        = "string"
    required    = false
    default     = ""
    description = "Assigned owner ID"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.hubapi.com/crm/v3/objects/deals"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }

    body {
      kind = "json"
      value = {
        properties = {
          dealname         = "{{ args.dealname }}"
          dealstage        = "{{ args.dealstage }}"
          pipeline         = "{{ args.pipeline }}"
          amount           = "{{ args.amount }}"
          closedate        = "{{ args.closedate }}"
          hubspot_owner_id = "{{ args.hubspot_owner_id }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Created deal {{ result.id }}: {{ result.properties.dealname }} — {{ result.properties.amount }}"
  }
}

command "create_ticket" {
  title       = "Create ticket"
  summary     = "Create a support ticket"
  description = "Create a new support ticket in HubSpot CRM."
  categories  = ["crm"]

  annotations {
    mode    = "write"
    secrets = ["hubspot.token"]
  }

  param "subject" {
    type        = "string"
    required    = true
    description = "Ticket title"
  }

  param "content" {
    type        = "string"
    required    = false
    default     = ""
    description = "Ticket description body"
  }

  param "hs_pipeline" {
    type        = "string"
    required    = false
    default     = "0"
    description = "Pipeline ID"
  }

  param "hs_pipeline_stage" {
    type        = "string"
    required    = false
    default     = "1"
    description = "Stage ID"
  }

  param "hs_ticket_priority" {
    type        = "string"
    required    = false
    default     = ""
    description = "Priority: LOW, MEDIUM, or HIGH"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.hubapi.com/crm/v3/objects/tickets"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }

    body {
      kind = "json"
      value = {
        properties = {
          subject            = "{{ args.subject }}"
          content            = "{{ args.content }}"
          hs_pipeline        = "{{ args.hs_pipeline }}"
          hs_pipeline_stage  = "{{ args.hs_pipeline_stage }}"
          hs_ticket_priority = "{{ args.hs_ticket_priority }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Created ticket {{ result.id }}: {{ result.properties.subject }} (priority: {{ result.properties.hs_ticket_priority }})"
  }
}

command "create_note" {
  title       = "Create note"
  summary     = "Create a note and associate it with a CRM record"
  description = "Create an engagement note in HubSpot and associate it with a contact, deal, company, or ticket. Use association_type_id: 202=contact, 214=deal, 190=company, 220=ticket."
  categories  = ["crm"]

  annotations {
    mode    = "write"
    secrets = ["hubspot.token"]
  }

  param "note_body" {
    type        = "string"
    required    = true
    description = "Note text content"
  }

  param "timestamp" {
    type        = "string"
    required    = true
    description = "Note timestamp in ISO 8601 format (e.g. 2024-01-15T10:30:00Z)"
  }

  param "associated_object_id" {
    type        = "string"
    required    = true
    description = "ID of the record to attach the note to"
  }

  param "association_type_id" {
    type        = "integer"
    required    = true
    description = "Association type ID (202=contact, 214=deal, 190=company, 220=ticket)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.hubapi.com/crm/v3/objects/notes"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }

    body {
      kind = "json"
      value = {
        properties = {
          hs_note_body = "{{ args.note_body }}"
          hs_timestamp = "{{ args.timestamp }}"
        }
        associations = [{
          to = {
            id = "{{ args.associated_object_id }}"
          }
          types = [{
            associationCategory = "HUBSPOT_DEFINED"
            associationTypeId   = "{{ args.association_type_id }}"
          }]
        }]
      }
    }
  }

  result {
    decode = "json"
    output = "Created note {{ result.id }} associated with record {{ args.associated_object_id }}."
  }
}

command "list_owners" {
  title       = "List owners"
  summary     = "List HubSpot users who can be assigned as owners"
  description = "Retrieve a list of HubSpot owners (users) who can be assigned to contacts, deals, and other CRM records."
  categories  = ["crm"]

  annotations {
    mode    = "read"
    secrets = ["hubspot.token"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Records per page"
  }

  param "after" {
    type        = "string"
    required    = false
    default     = ""
    description = "Pagination cursor from a previous response"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.hubapi.com/crm/v3/owners"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }

    query = {
      limit = "{{ args.limit }}"
      after = "{{ args.after }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.results | length }} owners.{% for o in result.results %}\n- [{{ o.id }}] {{ o.firstName }} {{ o.lastName }} <{{ o.email }}>{% endfor %}"
  }
}

command "list_pipelines" {
  title       = "List pipelines"
  summary     = "List pipelines and stages for deals or tickets"
  description = "Retrieve all pipelines and their stages for a given object type. Use object_type 'deals' for sales pipelines or 'tickets' for support pipelines."
  categories  = ["sales", "crm"]

  annotations {
    mode    = "read"
    secrets = ["hubspot.token"]
  }

  param "object_type" {
    type        = "string"
    required    = true
    description = "Object type: deals or tickets"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.hubapi.com/crm/v3/pipelines/{{ args.object_type }}"

    auth {
      kind   = "bearer"
      secret = "hubspot.token"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.results | length }} pipelines.{% for p in result.results %}\n{{ p.label }} [{{ p.id }}]{% for s in p.stages %}\n  - {{ s.label }} ({{ s.id }}){% endfor %}{% endfor %}"
  }
}
