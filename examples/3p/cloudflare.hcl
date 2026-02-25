version = 1
provider = "cloudflare"
categories = ["infrastructure", "dns", "cdn", "edge-compute"]

command "verify_token" {
  title       = "Verify API token"
  summary     = "Check that the configured API token is valid"
  description = "Verify the current Cloudflare API token and show its status and expiration."
  categories  = ["auth"]

  annotations {
    mode    = "read"
    secrets = ["cloudflare.api_token"]
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.cloudflare.com/client/v4/user/tokens/verify"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/result" }
    output = "Token status: {{ result.status }}\nExpires: {{ result.expires_on | default('never') }}"
  }
}

command "list_zones" {
  title       = "List zones"
  summary     = "List all zones (domains) in the account"
  description = "List zones in the Cloudflare account, optionally filtered by domain name or status."
  categories  = ["dns"]

  annotations {
    mode    = "read"
    secrets = ["cloudflare.api_token"]
  }

  param "name" {
    type        = "string"
    required    = false
    description = "Filter by domain name"
  }

  param "status" {
    type        = "string"
    required    = false
    description = "Filter by status: initializing, pending, active, moved"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Results per page (5-50)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.cloudflare.com/client/v4/zones"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    query = {
      name     = "{{ args.name }}"
      status   = "{{ args.status }}"
      per_page = "{{ args.per_page }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/result" }
    output = "Found {{ result | length }} zones:\n{% for z in result %}\n- {{ z.name }} ({{ z.status }}) — ID: {{ z.id }}\n{% endfor %}"
  }
}

command "get_zone" {
  title       = "Get zone details"
  summary     = "Get details for a specific zone by ID"
  description = "Retrieve detailed information about a Cloudflare zone including plan, nameservers, and status."
  categories  = ["dns"]

  annotations {
    mode    = "read"
    secrets = ["cloudflare.api_token"]
  }

  param "zone_id" {
    type        = "string"
    required    = true
    description = "Zone identifier"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.cloudflare.com/client/v4/zones/{{ args.zone_id }}"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/result" }
    output = "Zone: {{ result.name }}\nStatus: {{ result.status }}\nPlan: {{ result.plan.name }}\nNameservers: {{ result.name_servers | join(', ') }}\nCreated: {{ result.created_on }}"
  }
}

command "list_dns_records" {
  title       = "List DNS records"
  summary     = "List DNS records for a zone"
  description = "List all DNS records for a zone, optionally filtered by record type or name."
  categories  = ["dns"]

  annotations {
    mode    = "read"
    secrets = ["cloudflare.api_token"]
  }

  param "zone_id" {
    type        = "string"
    required    = true
    description = "Zone identifier"
  }

  param "type" {
    type        = "string"
    required    = false
    description = "Record type filter: A, AAAA, CNAME, MX, TXT, NS, SRV"
  }

  param "name" {
    type        = "string"
    required    = false
    description = "Record name filter"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Results per page (1-5000)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.cloudflare.com/client/v4/zones/{{ args.zone_id }}/dns_records"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    query = {
      type     = "{{ args.type }}"
      name     = "{{ args.name }}"
      per_page = "{{ args.per_page }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/result" }
    output = "Found {{ result | length }} DNS records:\n{% for r in result %}\n- {{ r.type }} {{ r.name }} → {{ r.content }} (TTL: {{ r.ttl }}, proxied: {{ r.proxied }})\n{% endfor %}"
  }
}

command "create_dns_record" {
  title       = "Create DNS record"
  summary     = "Create a new DNS record in a zone"
  description = "Create a DNS record (A, AAAA, CNAME, MX, TXT, etc.) in the specified zone."
  categories  = ["dns"]

  annotations {
    mode    = "write"
    secrets = ["cloudflare.api_token"]
  }

  param "zone_id" {
    type        = "string"
    required    = true
    description = "Zone identifier"
  }

  param "type" {
    type        = "string"
    required    = true
    description = "Record type: A, AAAA, CNAME, MX, TXT, NS, SRV"
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Record name (e.g. sub.example.com)"
  }

  param "content" {
    type        = "string"
    required    = true
    description = "Record value (e.g. IP address, target domain)"
  }

  param "ttl" {
    type        = "integer"
    required    = false
    default     = 1
    description = "TTL in seconds (60-86400, or 1 for auto)"
  }

  param "proxied" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Whether to proxy through Cloudflare"
  }

  param "comment" {
    type        = "string"
    required    = false
    description = "Admin comment for the record"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.cloudflare.com/client/v4/zones/{{ args.zone_id }}/dns_records"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        type    = "{{ args.type }}"
        name    = "{{ args.name }}"
        content = "{{ args.content }}"
        ttl     = "{{ args.ttl }}"
        proxied = "{{ args.proxied }}"
        comment = "{{ args.comment }}"
      }
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/result" }
    output = "Created {{ result.type }} record: {{ result.name }} → {{ result.content }}\nID: {{ result.id }}\nProxied: {{ result.proxied }}\nTTL: {{ result.ttl }}"
  }
}

command "update_dns_record" {
  title       = "Update DNS record"
  summary     = "Update an existing DNS record"
  description = "Patch a DNS record to update its content, TTL, proxy status, or other fields."
  categories  = ["dns"]

  annotations {
    mode    = "write"
    secrets = ["cloudflare.api_token"]
  }

  param "zone_id" {
    type        = "string"
    required    = true
    description = "Zone identifier"
  }

  param "dns_record_id" {
    type        = "string"
    required    = true
    description = "DNS record identifier"
  }

  param "content" {
    type        = "string"
    required    = true
    description = "Updated record value"
  }

  param "ttl" {
    type        = "integer"
    required    = false
    default     = 1
    description = "Updated TTL in seconds (60-86400, or 1 for auto)"
  }

  param "proxied" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Updated proxy status"
  }

  operation {
    protocol = "http"
    method   = "PATCH"
    url      = "https://api.cloudflare.com/client/v4/zones/{{ args.zone_id }}/dns_records/{{ args.dns_record_id }}"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        content = "{{ args.content }}"
        ttl     = "{{ args.ttl }}"
        proxied = "{{ args.proxied }}"
      }
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/result" }
    output = "Updated record {{ result.id }}:\n{{ result.type }} {{ result.name }} → {{ result.content }}\nProxied: {{ result.proxied }}, TTL: {{ result.ttl }}"
  }
}

command "delete_dns_record" {
  title       = "Delete DNS record"
  summary     = "Delete a DNS record from a zone"
  description = "Permanently delete a DNS record by its identifier."
  categories  = ["dns"]

  annotations {
    mode    = "write"
    secrets = ["cloudflare.api_token"]
  }

  param "zone_id" {
    type        = "string"
    required    = true
    description = "Zone identifier"
  }

  param "dns_record_id" {
    type        = "string"
    required    = true
    description = "DNS record identifier"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.cloudflare.com/client/v4/zones/{{ args.zone_id }}/dns_records/{{ args.dns_record_id }}"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/result" }
    output = "Deleted DNS record {{ result.id }}"
  }
}

command "purge_cache" {
  title       = "Purge cache"
  summary     = "Purge all cached content for a zone"
  description = "Purge the entire Cloudflare cache for a zone. This removes all cached resources and forces re-fetching from origin."
  categories  = ["cdn"]

  annotations {
    mode    = "write"
    secrets = ["cloudflare.api_token"]
  }

  param "zone_id" {
    type        = "string"
    required    = true
    description = "Zone identifier"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.cloudflare.com/client/v4/zones/{{ args.zone_id }}/purge_cache"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        purge_everything = "{{ true }}"
      }
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/result" }
    output = "Cache purge initiated. Operation ID: {{ result.id }}"
  }
}

command "edit_zone_setting" {
  title       = "Edit zone setting"
  summary     = "Update a zone setting like SSL mode or security level"
  description = "Modify a zone setting. Common settings: ssl (off/flexible/full/strict), security_level (essentially_off/low/medium/high/under_attack), always_use_https, min_tls_version, tls_1_3, browser_check."
  categories  = ["infrastructure"]

  annotations {
    mode    = "write"
    secrets = ["cloudflare.api_token"]
  }

  param "zone_id" {
    type        = "string"
    required    = true
    description = "Zone identifier"
  }

  param "setting_id" {
    type        = "string"
    required    = true
    description = "Setting ID (e.g. ssl, security_level, always_use_https, min_tls_version)"
  }

  param "value" {
    type        = "string"
    required    = true
    description = "New setting value (e.g. strict, high, on)"
  }

  operation {
    protocol = "http"
    method   = "PATCH"
    url      = "https://api.cloudflare.com/client/v4/zones/{{ args.zone_id }}/settings/{{ args.setting_id }}"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        value = "{{ args.value }}"
      }
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/result" }
    output = "Updated setting \"{{ result.id }}\" to \"{{ result.value }}\"\nModified: {{ result.modified_on }}"
  }
}

command "list_workers" {
  title       = "List Workers"
  summary     = "List Workers scripts for an account"
  description = "List all Cloudflare Workers scripts deployed in the specified account."
  categories  = ["edge-compute"]

  annotations {
    mode    = "read"
    secrets = ["cloudflare.api_token"]
  }

  param "account_id" {
    type        = "string"
    required    = true
    description = "Account identifier"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.cloudflare.com/client/v4/accounts/{{ args.account_id }}/workers/scripts"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/result" }
    output = "Found {{ result | length }} Workers:\n{% for w in result %}\n- {{ w.id }} (modified: {{ w.modified_on }})\n{% endfor %}"
  }
}

command "list_kv_namespaces" {
  title       = "List KV namespaces"
  summary     = "List Workers KV namespaces for an account"
  description = "List all Workers KV namespaces in the specified account."
  categories  = ["edge-compute"]

  annotations {
    mode    = "read"
    secrets = ["cloudflare.api_token"]
  }

  param "account_id" {
    type        = "string"
    required    = true
    description = "Account identifier"
  }

  param "per_page" {
    type        = "integer"
    required    = false
    default     = 20
    description = "Results per page (1-1000)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.cloudflare.com/client/v4/accounts/{{ args.account_id }}/storage/kv/namespaces"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    query = {
      per_page = "{{ args.per_page }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/result" }
    output = "Found {{ result | length }} KV namespaces:\n{% for ns in result %}\n- {{ ns.title }} — ID: {{ ns.id }}\n{% endfor %}"
  }
}

command "read_kv_pair" {
  title       = "Read KV value"
  summary     = "Read a value from a Workers KV namespace"
  description = "Read the value of a key from a Workers KV namespace. Returns the raw stored value."
  categories  = ["edge-compute"]

  annotations {
    mode    = "read"
    secrets = ["cloudflare.api_token"]
  }

  param "account_id" {
    type        = "string"
    required    = true
    description = "Account identifier"
  }

  param "namespace_id" {
    type        = "string"
    required    = true
    description = "KV namespace identifier"
  }

  param "key_name" {
    type        = "string"
    required    = true
    description = "Key name (max 512 bytes)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.cloudflare.com/client/v4/accounts/{{ args.account_id }}/storage/kv/namespaces/{{ args.namespace_id }}/values/{{ args.key_name }}"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Key \"{{ args.key_name }}\": {{ result }}"
  }
}

command "write_kv_pair" {
  title       = "Write KV value"
  summary     = "Write a key-value pair to a Workers KV namespace"
  description = "Store a value for a key in a Workers KV namespace. Optionally set an expiration."
  categories  = ["edge-compute"]

  annotations {
    mode    = "write"
    secrets = ["cloudflare.api_token"]
  }

  param "account_id" {
    type        = "string"
    required    = true
    description = "Account identifier"
  }

  param "namespace_id" {
    type        = "string"
    required    = true
    description = "KV namespace identifier"
  }

  param "key_name" {
    type        = "string"
    required    = true
    description = "Key name"
  }

  param "value" {
    type        = "string"
    required    = true
    description = "Value to store"
  }

  param "expiration_ttl" {
    type        = "integer"
    required    = false
    description = "TTL in seconds from now (minimum 60)"
  }

  operation {
    protocol = "http"
    method   = "PUT"
    url      = "https://api.cloudflare.com/client/v4/accounts/{{ args.account_id }}/storage/kv/namespaces/{{ args.namespace_id }}/values/{{ args.key_name }}"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    query = {
      expiration_ttl = "{{ args.expiration_ttl }}"
    }

    headers = {
      Content-Type = "text/plain"
    }

    body {
      kind = "json"
      value = {
        value = "{{ args.value }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Wrote key \"{{ args.key_name }}\" to namespace {{ args.namespace_id }}"
  }
}

command "list_pages_projects" {
  title       = "List Pages projects"
  summary     = "List Cloudflare Pages projects for an account"
  description = "List all Cloudflare Pages projects deployed in the specified account."
  categories  = ["infrastructure"]

  annotations {
    mode    = "read"
    secrets = ["cloudflare.api_token"]
  }

  param "account_id" {
    type        = "string"
    required    = true
    description = "Account identifier"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.cloudflare.com/client/v4/accounts/{{ args.account_id }}/pages/projects"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/result" }
    output = "Found {{ result | length }} Pages projects:\n{% for p in result %}\n- {{ p.name }} — {{ p.subdomain }}\n{% endfor %}"
  }
}

command "create_waf_rule" {
  title       = "Create WAF rule"
  summary     = "Create a custom WAF firewall rule"
  description = "Create a custom Web Application Firewall rule in a zone's ruleset. Requires the ruleset ID for the http_request_firewall_custom phase."
  categories  = ["infrastructure"]

  annotations {
    mode    = "write"
    secrets = ["cloudflare.api_token"]
  }

  param "zone_id" {
    type        = "string"
    required    = true
    description = "Zone identifier"
  }

  param "ruleset_id" {
    type        = "string"
    required    = true
    description = "Ruleset ID for the http_request_firewall_custom phase"
  }

  param "expression" {
    type        = "string"
    required    = true
    description = "Rule expression (e.g. '(ip.geoip.country eq \"BR\")')"
  }

  param "action" {
    type        = "string"
    required    = true
    description = "Action: block, challenge, js_challenge, managed_challenge, log, skip"
  }

  param "description" {
    type        = "string"
    required    = false
    description = "Human-readable rule description"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.cloudflare.com/client/v4/zones/{{ args.zone_id }}/rulesets/{{ args.ruleset_id }}/rules"

    auth {
      kind   = "bearer"
      secret = "cloudflare.api_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        expression  = "{{ args.expression }}"
        action      = "{{ args.action }}"
        description = "{{ args.description }}"
      }
    }
  }

  result {
    decode = "json"
    extract { json_pointer = "/result" }
    output = "Created WAF rule in ruleset {{ result.id }}\nRules count: {{ result.rules | length }}"
  }
}
