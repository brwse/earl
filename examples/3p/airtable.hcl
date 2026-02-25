version = 1
provider = "airtable"
categories = ["databases", "productivity", "collaboration"]

command "list_bases" {
  title       = "List bases"
  summary     = "List all accessible Airtable bases"
  description = "Returns a list of all bases the authenticated user has access to, with pagination support."
  categories  = ["databases"]

  annotations {
    mode    = "read"
    secrets = ["airtable.token"]
  }

  param "offset" {
    type        = "string"
    required    = false
    description = "Pagination cursor from a previous response"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.airtable.com/v0/meta/bases"

    auth {
      kind   = "bearer"
      secret = "airtable.token"
    }

    query = {
      offset = "{{ args.offset }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.bases | length }} bases:\n{% for base in result.bases %}- {{ base.name }} ({{ base.id }}) [{{ base.permissionLevel }}]\n{% endfor %}{% if result.offset %}(more results available){% endif %}"
  }
}

command "get_base_schema" {
  title       = "Get base schema"
  summary     = "Retrieve the schema of an Airtable base"
  description = "Returns all tables, fields, and views for the specified base."
  categories  = ["databases"]

  annotations {
    mode    = "read"
    secrets = ["airtable.token"]
  }

  param "base_id" {
    type        = "string"
    required    = true
    description = "Base ID (e.g. appXXX)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.airtable.com/v0/meta/bases/{{ args.base_id }}/tables"

    auth {
      kind   = "bearer"
      secret = "airtable.token"
    }
  }

  result {
    decode = "json"
    output = "Base has {{ result.tables | length }} tables:\n{% for table in result.tables %}- {{ table.name }} ({{ table.id }}): {{ table.fields | length }} fields, {{ table.views | length }} views\n{% endfor %}"
  }
}

command "list_records" {
  title       = "List records"
  summary     = "List records from an Airtable table"
  description = "Retrieve records from a table with optional filtering, sorting, and pagination. Returns up to 100 records per page."
  categories  = ["databases"]

  annotations {
    mode    = "read"
    secrets = ["airtable.token"]
  }

  param "base_id" {
    type        = "string"
    required    = true
    description = "Base ID (e.g. appXXX)"
  }

  param "table_id_or_name" {
    type        = "string"
    required    = true
    description = "Table ID (e.g. tblXXX) or URL-encoded table name"
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Number of records per page (max 100)"
  }

  param "offset" {
    type        = "string"
    required    = false
    description = "Pagination cursor from a previous response"
  }

  param "view" {
    type        = "string"
    required    = false
    description = "View name or ID to filter records by"
  }

  param "filter_by_formula" {
    type        = "string"
    required    = false
    description = "Airtable formula to filter records (e.g. {Status}='Done')"
  }

  param "max_records" {
    type        = "integer"
    required    = false
    description = "Maximum total number of records to return"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.airtable.com/v0/{{ args.base_id }}/{{ args.table_id_or_name }}"

    auth {
      kind   = "bearer"
      secret = "airtable.token"
    }

    query = {
      pageSize        = "{{ args.page_size }}"
      offset          = "{{ args.offset }}"
      view            = "{{ args.view }}"
      filterByFormula = "{{ args.filter_by_formula }}"
      maxRecords      = "{{ args.max_records }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.records | length }} records:\n{% for record in result.records %}- {{ record.id }}: {{ record.fields | tojson }}\n{% endfor %}{% if result.offset %}(more results available — offset: {{ result.offset }}){% endif %}"
  }
}

command "get_record" {
  title       = "Get record"
  summary     = "Retrieve a single record by ID"
  description = "Fetch a specific record from an Airtable table by its record ID."
  categories  = ["databases"]

  annotations {
    mode    = "read"
    secrets = ["airtable.token"]
  }

  param "base_id" {
    type        = "string"
    required    = true
    description = "Base ID (e.g. appXXX)"
  }

  param "table_id_or_name" {
    type        = "string"
    required    = true
    description = "Table ID or URL-encoded table name"
  }

  param "record_id" {
    type        = "string"
    required    = true
    description = "Record ID (e.g. recXXX)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.airtable.com/v0/{{ args.base_id }}/{{ args.table_id_or_name }}/{{ args.record_id }}"

    auth {
      kind   = "bearer"
      secret = "airtable.token"
    }
  }

  result {
    decode = "json"
    output = "Record {{ result.id }} (created {{ result.createdTime }}):\n{% for key, value in result.fields.items() %}  {{ key }}: {{ value }}\n{% endfor %}"
  }
}

command "create_records" {
  title       = "Create records"
  summary     = "Create new records in a table"
  description = "Create up to 10 records in a single request. Each record must include a fields object with field name/value pairs."
  categories  = ["databases"]

  annotations {
    mode    = "write"
    secrets = ["airtable.token"]
  }

  param "base_id" {
    type        = "string"
    required    = true
    description = "Base ID (e.g. appXXX)"
  }

  param "table_id_or_name" {
    type        = "string"
    required    = true
    description = "Table ID or URL-encoded table name"
  }

  param "records" {
    type        = "array"
    required    = true
    description = "Array of record objects, each with a 'fields' object (e.g. [{\"fields\": {\"Name\": \"Task 1\"}}]). Max 10."
  }

  param "typecast" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Auto-convert string values to correct field types"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.airtable.com/v0/{{ args.base_id }}/{{ args.table_id_or_name }}"

    auth {
      kind   = "bearer"
      secret = "airtable.token"
    }

    headers = {
      Content-Type = "application/json"
    }

    body {
      kind = "json"
      value = {
        records  = "{{ args.records }}"
        typecast = "{{ args.typecast }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created {{ result.records | length }} record(s):\n{% for record in result.records %}- {{ record.id }}: {{ record.fields | tojson }}\n{% endfor %}"
  }
}

command "update_records" {
  title       = "Update records"
  summary     = "Update existing records in a table"
  description = "Update up to 10 records using PATCH (partial update — only specified fields change). Each record must include an 'id' and 'fields' object."
  categories  = ["databases"]

  annotations {
    mode    = "write"
    secrets = ["airtable.token"]
  }

  param "base_id" {
    type        = "string"
    required    = true
    description = "Base ID (e.g. appXXX)"
  }

  param "table_id_or_name" {
    type        = "string"
    required    = true
    description = "Table ID or URL-encoded table name"
  }

  param "records" {
    type        = "array"
    required    = true
    description = "Array of record objects with 'id' and 'fields' (e.g. [{\"id\": \"recXXX\", \"fields\": {\"Status\": \"Done\"}}]). Max 10."
  }

  param "typecast" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Auto-convert string values to correct field types"
  }

  operation {
    protocol = "http"
    method   = "PATCH"
    url      = "https://api.airtable.com/v0/{{ args.base_id }}/{{ args.table_id_or_name }}"

    auth {
      kind   = "bearer"
      secret = "airtable.token"
    }

    headers = {
      Content-Type = "application/json"
    }

    body {
      kind = "json"
      value = {
        records  = "{{ args.records }}"
        typecast = "{{ args.typecast }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Updated {{ result.records | length }} record(s):\n{% for record in result.records %}- {{ record.id }}: {{ record.fields | tojson }}\n{% endfor %}"
  }
}

command "delete_record" {
  title       = "Delete record"
  summary     = "Delete a record from a table"
  description = "Permanently delete a single record from an Airtable table."
  categories  = ["databases"]

  annotations {
    mode    = "write"
    secrets = ["airtable.token"]
  }

  param "base_id" {
    type        = "string"
    required    = true
    description = "Base ID (e.g. appXXX)"
  }

  param "table_id_or_name" {
    type        = "string"
    required    = true
    description = "Table ID or URL-encoded table name"
  }

  param "record_id" {
    type        = "string"
    required    = true
    description = "Record ID to delete (e.g. recXXX)"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.airtable.com/v0/{{ args.base_id }}/{{ args.table_id_or_name }}/{{ args.record_id }}"

    auth {
      kind   = "bearer"
      secret = "airtable.token"
    }
  }

  result {
    decode = "json"
    output = "Deleted record {{ result.id }}: {{ result.deleted }}"
  }
}

command "create_table" {
  title       = "Create table"
  summary     = "Create a new table in a base"
  description = "Create a new table in the specified base with a name and initial field definitions."
  categories  = ["databases"]

  annotations {
    mode    = "write"
    secrets = ["airtable.token"]
  }

  param "base_id" {
    type        = "string"
    required    = true
    description = "Base ID (e.g. appXXX)"
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Name for the new table"
  }

  param "fields" {
    type        = "array"
    required    = true
    description = "Array of field definitions, each with 'name' and 'type' (e.g. [{\"name\": \"Name\", \"type\": \"singleLineText\"}])"
  }

  param "description" {
    type        = "string"
    required    = false
    description = "Optional table description"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.airtable.com/v0/meta/bases/{{ args.base_id }}/tables"

    auth {
      kind   = "bearer"
      secret = "airtable.token"
    }

    headers = {
      Content-Type = "application/json"
    }

    body {
      kind = "json"
      value = {
        name        = "{{ args.name }}"
        description = "{{ args.description }}"
        fields      = "{{ args.fields }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created table \"{{ result.name }}\" ({{ result.id }}) with {{ result.fields | length }} fields."
  }
}

command "create_field" {
  title       = "Create field"
  summary     = "Add a new field to a table"
  description = "Create a new field (column) in an existing Airtable table."
  categories  = ["databases"]

  annotations {
    mode    = "write"
    secrets = ["airtable.token"]
  }

  param "base_id" {
    type        = "string"
    required    = true
    description = "Base ID (e.g. appXXX)"
  }

  param "table_id" {
    type        = "string"
    required    = true
    description = "Table ID (e.g. tblXXX)"
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Field name"
  }

  param "type" {
    type        = "string"
    required    = true
    description = "Field type (e.g. singleLineText, number, singleSelect, multipleAttachments)"
  }

  param "description" {
    type        = "string"
    required    = false
    description = "Optional field description"
  }

  param "options" {
    type        = "object"
    required    = false
    description = "Type-specific options (e.g. select choices, number precision)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.airtable.com/v0/meta/bases/{{ args.base_id }}/tables/{{ args.table_id }}/fields"

    auth {
      kind   = "bearer"
      secret = "airtable.token"
    }

    headers = {
      Content-Type = "application/json"
    }

    body {
      kind = "json"
      value = {
        name        = "{{ args.name }}"
        type        = "{{ args.type }}"
        description = "{{ args.description }}"
        options     = "{{ args.options }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created field \"{{ result.name }}\" ({{ result.id }}) of type {{ result.type }}."
  }
}

command "list_comments" {
  title       = "List comments"
  summary     = "List comments on a record"
  description = "Retrieve all comments on a specific Airtable record."
  categories  = ["collaboration"]

  annotations {
    mode    = "read"
    secrets = ["airtable.token"]
  }

  param "base_id" {
    type        = "string"
    required    = true
    description = "Base ID (e.g. appXXX)"
  }

  param "table_id_or_name" {
    type        = "string"
    required    = true
    description = "Table ID or URL-encoded table name"
  }

  param "record_id" {
    type        = "string"
    required    = true
    description = "Record ID (e.g. recXXX)"
  }

  param "offset" {
    type        = "string"
    required    = false
    description = "Pagination cursor from a previous response"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.airtable.com/v0/{{ args.base_id }}/{{ args.table_id_or_name }}/{{ args.record_id }}/comments"

    auth {
      kind   = "bearer"
      secret = "airtable.token"
    }

    query = {
      offset = "{{ args.offset }}"
    }
  }

  result {
    decode = "json"
    output = "{{ result.comments | length }} comment(s) on record {{ args.record_id }}:\n{% for comment in result.comments %}- {{ comment.author.name }} ({{ comment.createdTime }}): {{ comment.text }}\n{% endfor %}"
  }
}

command "create_comment" {
  title       = "Create comment"
  summary     = "Add a comment to a record"
  description = "Post a new comment on a specific Airtable record."
  categories  = ["collaboration"]

  annotations {
    mode    = "write"
    secrets = ["airtable.token"]
  }

  param "base_id" {
    type        = "string"
    required    = true
    description = "Base ID (e.g. appXXX)"
  }

  param "table_id_or_name" {
    type        = "string"
    required    = true
    description = "Table ID or URL-encoded table name"
  }

  param "record_id" {
    type        = "string"
    required    = true
    description = "Record ID (e.g. recXXX)"
  }

  param "text" {
    type        = "string"
    required    = true
    description = "Comment text"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.airtable.com/v0/{{ args.base_id }}/{{ args.table_id_or_name }}/{{ args.record_id }}/comments"

    auth {
      kind   = "bearer"
      secret = "airtable.token"
    }

    headers = {
      Content-Type = "application/json"
    }

    body {
      kind = "json"
      value = {
        text = "{{ args.text }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Comment created by {{ result.author.name }} at {{ result.createdTime }}: {{ result.text }}"
  }
}

command "list_webhooks" {
  title       = "List webhooks"
  summary     = "List all webhooks for a base"
  description = "Retrieve all registered webhooks for the specified Airtable base."
  categories  = ["databases"]

  annotations {
    mode    = "read"
    secrets = ["airtable.token"]
  }

  param "base_id" {
    type        = "string"
    required    = true
    description = "Base ID (e.g. appXXX)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.airtable.com/v0/bases/{{ args.base_id }}/webhooks"

    auth {
      kind   = "bearer"
      secret = "airtable.token"
    }
  }

  result {
    decode = "json"
    output = "{{ result.webhooks | length }} webhook(s):\n{% for wh in result.webhooks %}- {{ wh.id }}: {{ wh.notificationUrl }} (expires {{ wh.expirationTime }})\n{% endfor %}"
  }
}

command "create_webhook" {
  title       = "Create webhook"
  summary     = "Register a new webhook for a base"
  description = "Create a webhook to receive notifications when data changes in an Airtable base. Webhooks expire after 7 days."
  categories  = ["databases"]

  annotations {
    mode    = "write"
    secrets = ["airtable.token"]
  }

  param "base_id" {
    type        = "string"
    required    = true
    description = "Base ID (e.g. appXXX)"
  }

  param "notification_url" {
    type        = "string"
    required    = true
    description = "URL to receive webhook payloads"
  }

  param "specification" {
    type        = "object"
    required    = true
    description = "Webhook specification with options defining which changes to watch (e.g. {\"options\": {\"filters\": {\"dataTypes\": [\"tableData\"]}}})"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.airtable.com/v0/bases/{{ args.base_id }}/webhooks"

    auth {
      kind   = "bearer"
      secret = "airtable.token"
    }

    headers = {
      Content-Type = "application/json"
    }

    body {
      kind = "json"
      value = {
        notificationUrl = "{{ args.notification_url }}"
        specification   = "{{ args.specification }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Webhook created: {{ result.id }}\nSecret (base64): {{ result.macSecretBase64 }}\nExpires: {{ result.expirationTime }}"
  }
}
