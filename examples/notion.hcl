version = 1
provider = "notion"
categories = ["productivity", "knowledge-management"]

command "search" {
  title       = "Search"
  summary     = "Search pages and databases by title"
  description = "Search across all pages and databases accessible to the integration. Matches against titles only, not page content."
  categories  = ["search"]

  annotations {
    mode    = "read"
    secrets = ["notion.token"]
  }

  param "query" {
    type        = "string"
    required    = false
    default     = ""
    description = "Text to match against page and database titles"
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of results per page (max 100)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.notion.com/v1/search"

    auth {
      kind   = "bearer"
      secret = "notion.token"
    }

    headers = {
      Notion-Version = "2022-06-28"
    }

    body {
      kind = "json"
      value = {
        query     = "{{ args.query }}"
        page_size = "{{ args.page_size }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.results | length }} results."
  }
}

command "query_database" {
  title       = "Query database"
  summary     = "Query a database with optional filters and sorts"
  description = "Retrieve pages from a Notion database, optionally filtered and sorted."
  categories  = ["databases"]

  annotations {
    mode    = "read"
    secrets = ["notion.token"]
  }

  param "database_id" {
    type        = "string"
    required    = true
    description = "UUID of the database to query"
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of results per page (max 100)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.notion.com/v1/databases/{{ args.database_id }}/query"

    auth {
      kind   = "bearer"
      secret = "notion.token"
    }

    headers = {
      Notion-Version = "2022-06-28"
    }

    body {
      kind = "json"
      value = {
        page_size = "{{ args.page_size }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Database query returned {{ result.results | length }} pages."
  }
}

command "get_database" {
  title       = "Get database"
  summary     = "Retrieve a database by ID"
  description = "Fetch metadata and schema for a Notion database including its properties."
  categories  = ["databases"]

  annotations {
    mode    = "read"
    secrets = ["notion.token"]
  }

  param "database_id" {
    type        = "string"
    required    = true
    description = "UUID of the database"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.notion.com/v1/databases/{{ args.database_id }}"

    auth {
      kind   = "bearer"
      secret = "notion.token"
    }

    headers = {
      Notion-Version = "2022-06-28"
    }
  }

  result {
    decode = "json"
    output = "Database: {{ result.id }} — Created: {{ result.created_time }}, Last edited: {{ result.last_edited_time }}"
  }
}

command "create_page" {
  title       = "Create page"
  summary     = "Create a new page in a database or as a child of another page"
  description = "Create a Notion page. Specify a parent database or page, properties, and optional content blocks."
  categories  = ["pages"]

  annotations {
    mode    = "write"
    secrets = ["notion.token"]
  }

  param "parent" {
    type        = "object"
    required    = true
    description = "Parent object, e.g. {\"database_id\": \"...\"} or {\"page_id\": \"...\"}"
  }

  param "properties" {
    type        = "object"
    required    = true
    description = "Page properties matching the parent database schema or a title property"
  }

  param "children" {
    type        = "array"
    required    = false
    description = "Array of block objects for page content (max 100)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.notion.com/v1/pages"

    auth {
      kind   = "bearer"
      secret = "notion.token"
    }

    headers = {
      Notion-Version = "2022-06-28"
    }

    body {
      kind = "json"
      value = {
        parent     = "{{ args.parent }}"
        properties = "{{ args.properties }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created page: {{ result.id }} — {{ result.url }}"
  }
}

command "get_page" {
  title       = "Get page"
  summary     = "Retrieve a page by ID"
  description = "Fetch metadata and properties for a Notion page."
  categories  = ["pages"]

  annotations {
    mode    = "read"
    secrets = ["notion.token"]
  }

  param "page_id" {
    type        = "string"
    required    = true
    description = "UUID of the page"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.notion.com/v1/pages/{{ args.page_id }}"

    auth {
      kind   = "bearer"
      secret = "notion.token"
    }

    headers = {
      Notion-Version = "2022-06-28"
    }
  }

  result {
    decode = "json"
    output = "Page: {{ result.id }} — Created: {{ result.created_time }}, Last edited: {{ result.last_edited_time }}, Archived: {{ result.archived }}, URL: {{ result.url }}"
  }
}

command "update_page" {
  title       = "Update page"
  summary     = "Update properties or archive a page"
  description = "Update a Notion page's properties, icon, cover, or archive status."
  categories  = ["pages"]

  annotations {
    mode    = "write"
    secrets = ["notion.token"]
  }

  param "page_id" {
    type        = "string"
    required    = true
    description = "UUID of the page to update"
  }

  param "properties" {
    type        = "object"
    required    = false
    description = "Property values to update"
  }

  param "archived" {
    type        = "boolean"
    required    = false
    description = "Set to true to archive the page, false to unarchive"
  }

  operation {
    protocol = "http"
    method   = "PATCH"
    url      = "https://api.notion.com/v1/pages/{{ args.page_id }}"

    auth {
      kind   = "bearer"
      secret = "notion.token"
    }

    headers = {
      Notion-Version = "2022-06-28"
    }

    body {
      kind = "json"
      value = {
        properties = "{{ args.properties }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Updated page: {{ result.id }} — Last edited: {{ result.last_edited_time }}, URL: {{ result.url }}"
  }
}

command "get_block_children" {
  title       = "Get block children"
  summary     = "Retrieve child blocks of a block or page"
  description = "List the child blocks of a given block or page. Use this to read page content."
  categories  = ["blocks"]

  annotations {
    mode    = "read"
    secrets = ["notion.token"]
  }

  param "block_id" {
    type        = "string"
    required    = true
    description = "UUID of the block or page"
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Number of blocks to return (max 100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.notion.com/v1/blocks/{{ args.block_id }}/children"

    auth {
      kind   = "bearer"
      secret = "notion.token"
    }

    query = {
      page_size = "{{ args.page_size }}"
    }

    headers = {
      Notion-Version = "2022-06-28"
    }
  }

  result {
    decode = "json"
    output = "{{ result.results | length }} child blocks returned."
  }
}

command "append_blocks" {
  title       = "Append blocks"
  summary     = "Append content blocks to a page or block"
  description = "Append child blocks to a parent block or page. Use this to add content to a page."
  categories  = ["blocks"]

  annotations {
    mode    = "write"
    secrets = ["notion.token"]
  }

  param "block_id" {
    type        = "string"
    required    = true
    description = "UUID of the parent block or page"
  }

  param "children" {
    type        = "array"
    required    = true
    description = "Array of block objects to append (max 100)"
  }

  operation {
    protocol = "http"
    method   = "PATCH"
    url      = "https://api.notion.com/v1/blocks/{{ args.block_id }}/children"

    auth {
      kind   = "bearer"
      secret = "notion.token"
    }

    headers = {
      Notion-Version = "2022-06-28"
    }

    body {
      kind = "json"
      value = {
        children = "{{ args.children }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Appended {{ result.results | length }} blocks."
  }
}

command "delete_block" {
  title       = "Delete block"
  summary     = "Delete a block by ID"
  description = "Delete (archive) a block. This also deletes all child blocks."
  categories  = ["blocks"]

  annotations {
    mode    = "write"
    secrets = ["notion.token"]
  }

  param "block_id" {
    type        = "string"
    required    = true
    description = "UUID of the block to delete"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://api.notion.com/v1/blocks/{{ args.block_id }}"

    auth {
      kind   = "bearer"
      secret = "notion.token"
    }

    headers = {
      Notion-Version = "2022-06-28"
    }
  }

  result {
    decode = "json"
    output = "Deleted block: {{ result.id }} (type: {{ result.type }})"
  }
}

command "list_users" {
  title       = "List users"
  summary     = "List all users in the workspace"
  description = "Retrieve all users (people and bots) in the Notion workspace."
  categories  = ["users"]

  annotations {
    mode    = "read"
    secrets = ["notion.token"]
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Number of users to return (max 100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.notion.com/v1/users"

    auth {
      kind   = "bearer"
      secret = "notion.token"
    }

    query = {
      page_size = "{{ args.page_size }}"
    }

    headers = {
      Notion-Version = "2022-06-28"
    }
  }

  result {
    decode = "json"
    output = "{{ result.results | length }} users returned."
  }
}

command "create_comment" {
  title       = "Create comment"
  summary     = "Add a comment to a page"
  description = "Create a new comment on a Notion page."
  categories  = ["comments"]

  annotations {
    mode    = "write"
    secrets = ["notion.token"]
  }

  param "page_id" {
    type        = "string"
    required    = true
    description = "UUID of the page to comment on"
  }

  param "text" {
    type        = "string"
    required    = true
    description = "Plain text content of the comment"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.notion.com/v1/comments"

    auth {
      kind   = "bearer"
      secret = "notion.token"
    }

    headers = {
      Notion-Version = "2022-06-28"
    }

    body {
      kind = "json"
      value = {
        parent = {
          page_id = "{{ args.page_id }}"
        }
        rich_text = [
          {
            type = "text"
            text = {
              content = "{{ args.text }}"
            }
          }
        ]
      }
    }
  }

  result {
    decode = "json"
    output = "Created comment: {{ result.id }} in discussion {{ result.discussion_id }}"
  }
}

command "list_comments" {
  title       = "List comments"
  summary     = "List comments on a page or block"
  description = "Retrieve all comments on a Notion page or block."
  categories  = ["comments"]

  annotations {
    mode    = "read"
    secrets = ["notion.token"]
  }

  param "block_id" {
    type        = "string"
    required    = true
    description = "UUID of the page or block to list comments for"
  }

  param "page_size" {
    type        = "integer"
    required    = false
    default     = 100
    description = "Number of comments to return (max 100)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.notion.com/v1/comments"

    auth {
      kind   = "bearer"
      secret = "notion.token"
    }

    query = {
      block_id  = "{{ args.block_id }}"
      page_size = "{{ args.page_size }}"
    }

    headers = {
      Notion-Version = "2022-06-28"
    }
  }

  result {
    decode = "json"
    output = "{{ result.results | length }} comments returned."
  }
}
